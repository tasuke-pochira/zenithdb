# ZenithDB 🚀

ZenithDB is a high-performance, persistent key-value database built from scratch in Rust. It is designed as a learning project to explore the core concepts of modern database architecture, specifically focusing on Log-Structured Merge-Trees (LSM Trees).

## Features ✨

* **LSM Tree Architecture:** Core design is based on an LSM Tree, optimized for high write throughput. Data is written to an in-memory `MemTable` and flushed to immutable `SSTable` files on disk.
* **Durability and Crash Safety:** A **Write-Ahead Log (WAL)** ensures that no data is lost in the event of a server crash. On startup, the server replays the log to restore its in-memory state.
* **Fast Reads with Sparse Index:** Each `SSTable` includes a sparse index, allowing for fast lookups without scanning the entire file. This turns O(N) scans into much faster, targeted reads.
* **Optimized Lookups with Bloom Filters:** `SSTables` are equipped with Bloom filters to rapidly check for the non-existence of a key, avoiding unnecessary disk I/O for keys that are not present.
* **Deletion and Compaction:** Supports key deletion using tombstones. A compaction process merges smaller `SSTable` files into larger ones, cleaning up old and deleted data to save space and improve read performance.
* **Asynchronous Server:** Built with `tokio`, the server is fully asynchronous and can handle thousands of concurrent client connections efficiently.
* **Client-Server Model:** Includes a dedicated client library (`zenithdb-client`) that provides a simple, high-level API for interacting with the database server.
* **Cargo Workspace:** The project is organized as a Cargo Workspace, cleanly separating the server, client, and example binaries.

## TODO List 📝

-   [ ] **Formalize Network Protocol:** Upgrade the simple text-based protocol to a more robust standard like **RESP (REdis Serialization Protocol)**.
-   [ ] **Improve Concurrency:** Replace the global locks on the `MemTable` and `WAL` with more granular or lock-free techniques to reduce contention under heavy load.
-   [ ] **Configuration:** Move hardcoded values (like `MemTable` size and index stride) into a configuration file.
-   [ ] **Metrics and Observability:** Add a basic metrics layer to track database performance (e.g., number of keys, flush duration, compaction stats).
-   [ ] **Multi-threaded Compaction:** Move the compaction process to a dedicated background thread pool so it doesn't interfere with foreground operations.
-   [ ] **Range Scans:** Implement support for scanning a range of keys, not just getting individual keys.