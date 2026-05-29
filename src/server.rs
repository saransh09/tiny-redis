use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::command::Command;
use crate::store::Store;

type SharedStore = Arc<Mutex<Store>>;

pub async fn run(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on {}", addr);

    let store = Arc::new(Mutex::new(Store::new()));

    loop {
        let (stream, client_addr) = listener.accept().await?;
        println!("Client connected {}", client_addr);

        let store = Arc::clone(&store);

        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, store).await {
                eprintln!("client error: {}", err);
            }
        });
    }
}

pub async fn handle_client(stream: TcpStream, store: SharedStore) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        let bytes_read = reader.read_line(&mut line).await?;

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
            Err(err) => err,
        };

        writer.write_all(response.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn handles_ping_command() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let store = Arc::new(Mutex::new(Store::new()));

        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_client(stream, store).await.unwrap();
        });

        let stream = TcpStream::connect(addr).await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        writer.write_all(b"PING\n").await.unwrap();

        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();

        assert_eq!(response.trim(), "PONG");

        drop(writer);
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn handles_set_and_get_commands() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let store = Arc::new(Mutex::new(Store::new()));

        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_client(stream, store).await.unwrap();
        });

        let stream = TcpStream::connect(addr).await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        writer.write_all(b"SET name alice\n").await.unwrap();

        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();
        assert_eq!(response.trim(), "OK");

        response.clear();

        writer.write_all(b"GET name\n").await.unwrap();

        reader.read_line(&mut response).await.unwrap();
        assert_eq!(response.trim(), "alice");

        drop(writer);
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn two_clients_share_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let store = Arc::new(Mutex::new(Store::new()));

        let server_store = Arc::clone(&store);

        let server_task = tokio::spawn(async move {
            for _ in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let store = Arc::clone(&server_store);

                tokio::spawn(async move {
                    handle_client(stream, store).await.unwrap();
                });
            }
        });

        let client1 = TcpStream::connect(addr).await.unwrap();
        let client2 = TcpStream::connect(addr).await.unwrap();

        let (reader1, mut writer1) = client1.into_split();
        let mut reader1 = BufReader::new(reader1);

        let (reader2, mut writer2) = client2.into_split();
        let mut reader2 = BufReader::new(reader2);

        let mut response = String::new();

        writer1.write_all(b"SET shared hello\n").await.unwrap();
        reader1.read_line(&mut response).await.unwrap();
        assert_eq!(response.trim(), "OK");

        response.clear();

        writer2.write_all(b"GET shared\n").await.unwrap();
        reader2.read_line(&mut response).await.unwrap();
        assert_eq!(response.trim(), "hello");

        drop(writer1);
        drop(writer2);

        server_task.await.unwrap();
    }
}
