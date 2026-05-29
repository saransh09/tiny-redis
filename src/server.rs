use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, broadcast};
use tokio::time::timeout;

use crate::command::Command;
use crate::store::Store;

type SharedStore = Arc<Mutex<Store>>;

pub async fn run(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on {}", addr);

    let store = Arc::new(Mutex::new(Store::new()));
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    let mut client_tasks = Vec::new();

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, client_addr) = listener.accept().await?;
                println!("Client connected {}", client_addr);

                let store = Arc::clone(&store);
                let shutdown_rx = shutdown_tx.subscribe();

                let task = tokio::spawn(async move {
                    if let Err(err) = handle_client(stream, store, shutdown_rx).await {
                        eprintln!("client error: {}", err);
                    }
                });

                client_tasks.push(task);
            }

            result = tokio::signal::ctrl_c() => {
                match result {
                    Ok(()) => {
                        println!("Shutdown signal received");
                    }
                    Err(err) => {
                        eprintln!("Failed to listen for shutdown signal: {}", err);
                    }
                }
                break;
            }
        }
    }

    println!("Notifying clients about shutdown");

    let _ = shutdown_tx.send(());
    println!("Waiting for client tasks to finish");

    for task in client_tasks {
        match timeout(Duration::from_secs(5), task).await {
            Ok(join_result) => {
                if let Err(err) = join_result {
                    eprintln!("client task failed to join: {}", err);
                }
            }
            Err(_) => {
                eprintln!("client task did not finish before timeout");
            }
        }
    }
    println!("Server shutting down");
    Ok(())
}

pub async fn handle_client(
    stream: TcpStream,
    store: SharedStore,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        tokio::select! {
            result = reader.read_line(&mut line) => {
                let bytes_read = result?;

                if bytes_read == 0 {
                    break;
                }
                // The lock is dropped immediately after accessing the store
                // This is good practice because you don't want to hold the
                // lock for too long.
                let response = match Command::parse(&line) {
                    Ok(command) => {
                        let mut store = store.lock().await;
                        store.execute(command)
                    }
                    Err(err) => err.response().to_string(),
                };

                writer.write_all(response.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }

            result = shutdown_rx.recv() => {
                match result {
                    Ok(()) => {
                        writer.write_all(b"SERVER shutting down\n").await?;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        writer.write_all(b"SERVER shutting down\n").await?;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        writer.write_all(b"SERVER shutting down\n").await?;
                    }
                }
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{TcpListener, TcpStream};

    async fn send_command(
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        command: &str,
    ) -> String {
        writer.write_all(command.as_bytes()).await.unwrap();
        writer.write_all(b"\n").await.unwrap();

        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();

        response.trim().to_string()
    }

    async fn start_test_client() -> (
        BufReader<tokio::net::tcp::OwnedReadHalf>,
        tokio::net::tcp::OwnedWriteHalf,
        tokio::task::JoinHandle<()>,
        broadcast::Sender<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let store = Arc::new(Mutex::new(Store::new()));
        let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);

        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_client(stream, store, shutdown_rx).await.unwrap();
        });

        let stream = TcpStream::connect(addr).await.unwrap();
        let (reader, writer) = stream.into_split();

        (BufReader::new(reader), writer, server_task, _shutdown_tx)
    }

    #[tokio::test]
    async fn handles_ping_command() {
        let (mut reader, mut writer, server_task, shutdown_tx) = start_test_client().await;

        let response = send_command(&mut reader, &mut writer, "PING").await;

        assert_eq!(response, "PONG");

        drop(writer);
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn handles_set_and_get_commands() {
        let (mut reader, mut writer, server_task, shutdown_tx) = start_test_client().await;

        let response = send_command(&mut reader, &mut writer, "SET name alice").await;
        assert_eq!(response, "OK");

        let response = send_command(&mut reader, &mut writer, "GET name").await;
        assert_eq!(response, "alice");

        drop(writer);
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn handles_unknown_command() {
        let (mut reader, mut writer, server_task, shutdown_tx) = start_test_client().await;

        let response = send_command(&mut reader, &mut writer, "BANANA").await;

        assert_eq!(response, "ERR unknown command");

        drop(writer);
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn handles_keys_and_flushall_commands() {
        let (mut reader, mut writer, server_task, shutdown_tx) = start_test_client().await;

        let response = send_command(&mut reader, &mut writer, "SET name alice").await;
        assert_eq!(response, "OK");

        let response = send_command(&mut reader, &mut writer, "KEYS").await;
        assert_eq!(response, "name");

        let response = send_command(&mut reader, &mut writer, "FLUSHALL").await;
        assert_eq!(response, "OK");

        let response = send_command(&mut reader, &mut writer, "GET name").await;
        assert_eq!(response, "(nil)");

        drop(writer);
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn two_clients_share_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let store = Arc::new(Mutex::new(Store::new()));
        let server_store = Arc::clone(&store);

        let (shutdown_tx, _) = broadcast::channel(1);
        let server_shutdown_tx = shutdown_tx.clone();

        let server_task = tokio::spawn(async move {
            for _ in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let store = Arc::clone(&server_store);
                let shutdown_rx = server_shutdown_tx.subscribe();

                tokio::spawn(async move {
                    handle_client(stream, store, shutdown_rx).await.unwrap();
                });
            }
        });

        let client1 = TcpStream::connect(addr).await.unwrap();
        let client2 = TcpStream::connect(addr).await.unwrap();

        let (reader1, mut writer1) = client1.into_split();
        let mut reader1 = BufReader::new(reader1);

        let (reader2, mut writer2) = client2.into_split();
        let mut reader2 = BufReader::new(reader2);

        let response = send_command(&mut reader1, &mut writer1, "SET shared hello").await;
        assert_eq!(response, "OK");

        let response = send_command(&mut reader2, &mut writer2, "GET shared").await;
        assert_eq!(response, "hello");

        drop(writer1);
        drop(writer2);

        server_task.await.unwrap();
    }
}
