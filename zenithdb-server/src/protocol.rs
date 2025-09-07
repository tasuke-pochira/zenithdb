/// Represents the commands that clients can send to the server.
#[derive(Debug)]
pub enum Command {
    Set { key: String, value: String },
    Get { key: String },
    Delete { key: String },
    Compact,
}

/// Represents the possible responses the server can send back.
#[derive(Debug)]
pub enum Response {
    Ok,
    Value(Option<String>),
    Error(String),
}

impl Command {
    /// Parses a raw string slice into a Command.
    pub fn from_str(s: &str) -> Result<Command, &'static str> {
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        match parts.as_slice() {
            ["SET", key, value] => Ok(Command::Set {
                key: key.to_string(),
                value: value.to_string(),
            }),
            ["GET", key] => Ok(Command::Get { key: key.to_string() }),
            ["DELETE", key] => Ok(Command::Delete { key: key.to_string() }),
            ["COMPACT"] => Ok(Command::Compact),
            _ => Err("Invalid command format"),
        }
    }
}