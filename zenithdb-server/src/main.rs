mod protocol;
mod storage;
mod bloom;

use crate::protocol::{Command, Response};
use std::sync::Arc;
use storage::StorageEngine;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    println!("🚀 ZenithDB server starting...");

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let db = match StorageEngine::new() {
        Ok(engine) => Arc::new(engine),
        Err(e) => {
            eprintln!("Fatal: Failed to initialize storage engine: {}", e);
            return;
        }
    };

    println!("✅ Server listening on 127.0.0.1:8080");

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("📥 Accepted new connection from: {}", addr);

        let db_clone = db.clone();

        tokio::spawn(async move {
            handle_connection(socket, db_clone).await;
        });
    }
}

async fn handle_connection(mut socket: TcpStream, db: Arc<StorageEngine>) {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => return,
            Ok(_) => {
                let response = match Command::from_str(&line) {
                    Ok(cmd) => execute_command(cmd, &db).await,
                    Err(e) => Response::Error(e.to_string()),
                };

                let response_bytes = match response {
                    Response::Ok => b"OK\n".to_vec(),
                    Response::Value(Some(v)) => format!("{}\n", v).into_bytes(),
                    Response::Value(None) => b"NULL\n".to_vec(),
                    Response::Error(e) => format!("ERROR: {}\n", e).into_bytes(),
                };

                if writer.write_all(&response_bytes).await.is_err() {
                    return;
                }
            }
            Err(e) => {
                eprintln!("Failed to read from socket; err = {:?}", e);
                return;
            }
        }
    }
}

async fn execute_command(cmd: Command, db: &Arc<StorageEngine>) -> Response {
    match cmd {
        Command::Set { key, value } => match db.set(key, value) {
            Ok(_) => Response::Ok,
            Err(e) => Response::Error(e.to_string()),
        },
        Command::Get { key } => Response::Value(db.get(&key)),
        Command::Delete { key } => match db.delete(key) {
            Ok(_) => Response::Ok,
            Err(e) => Response::Error(e.to_string()),
        },
        Command::Compact => match db.compact() {
            Ok(_) => Response::Ok,
            Err(e) => Response::Error(e.to_string()),
        },
    }
}