use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::command::Command;
use crate::store::Store;

pub async fn run(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on {}", addr);

    let (stream, client_addr) = listener.accept().await?;
    println!("Client connected {}", client_addr);

    handle_client(stream).await
}

pub async fn handle_client(stream: TcpStream) -> std::io::Result<()> {
    let mut store = Store::new();

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            println!("client disconnected");
            break;
        }

        let response = match Command::parse(&line) {
            Ok(command) => store.execute(command),
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

        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_client(stream).await.unwrap();
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

        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_client(stream).await.unwrap();
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
}
