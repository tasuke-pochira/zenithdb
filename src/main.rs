use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use dashmap::DashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("🚀 ZenithDB server starting...");

    // Bind the server to an address.
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    // Create our thread-safe, in-memory database.
    // Arc allows multiple threads to safely share ownership of the DashMap.
    let db = Arc::new(DashMap::<String, String>::new());
    println!("✅ Server listening on 127.0.0.1:8080");

    loop {
        // Wait for a new client connection.
        let (socket, addr) = listener.accept().await.unwrap();
        println!("📥 Accepted new connection from: {}", addr);

        // Clone the database handle for the new task.
        let db_clone = db.clone();

        // Spawn a new asynchronous task to handle this one client.
        // This allows the server to handle many clients at once.
        tokio::spawn(async move {
            handle_connection(socket, db_clone).await;
        });
    }
}

/// Handles a single client connection.
async fn handle_connection(mut socket: TcpStream, db: Arc<DashMap<String, String>>) {
    // Create a buffer to hold incoming data.
    let mut buf = [0; 1024];

    loop {
        // Read data from the client.
        let n = match socket.read(&mut buf).await {
            // Socket closed.
            Ok(0) => return,
            Ok(n) => n,
            Err(e) => {
                eprintln!("Failed to read from socket; err = {:?}", e);
                return;
            }
        };

        // Convert the received bytes into a string command.
        let command_str = String::from_utf8_lossy(&buf[0..n]);
        let parts: Vec<&str> = command_str.trim().split_whitespace().collect();

        // Parse the command.
        match parts.as_slice() {
            ["SET", key, value] => {
                db.insert(key.to_string(), value.to_string());
                if socket.write_all(b"OK\n").await.is_err() { return; }
            },
            ["GET", key] => {
                let response = match db.get(*key) {
                    Some(val) => format!("{}\n", val.value()),
                    None => "NULL\n".to_string(),
                };
                if socket.write_all(response.as_bytes()).await.is_err() { return; }
            },
            _ => {
                if socket.write_all(b"ERROR: Invalid command\n").await.is_err() { return; }
            }
        }
    }
}