// In examples/basic-client/src/main.rs

use zenithdb_client::{Client, ClientError};

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    println!("Connecting to ZenithDB server...");

    // Connect to the server.
    let mut client = Client::connect("127.0.0.1:8080").await?;
    println!("✅ Connection successful!");

    // --- SET a value ---
    let key1 = "hello";
    let value1 = "world";
    println!("\n> SET {} {}", key1, value1);
    client.set(key1, value1).await?;
    println!("Response: OK");

    // --- SET another value ---
    let key2 = "city";
    let value2 = "Chennai";
    println!("\n> SET {} {}", key2, value2);
    client.set(key2, value2).await?;
    println!("Response: OK");

    // --- GET the first value ---
    println!("\n> GET {}", key1);
    match client.get(key1).await? {
        Some(value) => println!("Response: {}", value),
        None => println!("Response: NULL"),
    }

    // --- GET a non-existent value ---
    let key3 = "nonexistent_key";
    println!("\n> GET {}", key3);
    match client.get(key3).await? {
        Some(value) => println!("Response: {}", value),
        None => println!("Response: NULL"),
    }

    Ok(())
}