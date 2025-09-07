use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};
use tokio::net::TcpStream;

/// Represents a connection to the ZenithDB server.
pub struct Client {
    // CORRECTED TYPES: Use the generic ReadHalf and WriteHalf from tokio::io
    reader: BufReader<ReadHalf<TcpStream>>,
    writer: WriteHalf<TcpStream>,
}

/// Represents the possible errors that can occur.
#[derive(Debug)]
pub enum ClientError {
    Io(std::io::Error),
    Server(String),
}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> Self {
        ClientError::Io(e)
    }
}

impl Client {
    /// Establishes a connection to the ZenithDB server at the given address.
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let stream = TcpStream::connect(addr).await?;
        
        // This function returns tokio::io::ReadHalf and tokio::io::WriteHalf
        let (reader, writer) = tokio::io::split(stream);

        Ok(Self {
            reader: BufReader::new(reader),
            writer,
        })
    }

    /// Sets a key-value pair in the database.
    pub async fn set(&mut self, key: &str, value: &str) -> Result<(), ClientError> {
        let command = format!("SET {} {}\n", key, value);
        self.writer.write_all(command.as_bytes()).await?;
        
        let mut response_buf = String::new();
        self.reader.read_line(&mut response_buf).await?;
        
        if response_buf.trim() == "OK" {
            Ok(())
        } else {
            Err(ClientError::Server(response_buf.trim().to_string()))
        }
    }

    /// Gets a value for a given key from the database.
    pub async fn get(&mut self, key: &str) -> Result<Option<String>, ClientError> {
        let command = format!("GET {}\n", key);
        self.writer.write_all(command.as_bytes()).await?;
        
        let mut response_buf = String::new();
        self.reader.read_line(&mut response_buf).await?;
        
        let trimmed_response = response_buf.trim();
        if trimmed_response == "NULL" {
            Ok(None)
        } else if trimmed_response.starts_with("ERROR:") {
            Err(ClientError::Server(trimmed_response.to_string()))
        } else {
            Ok(Some(trimmed_response.to_string()))
        }
    }
}