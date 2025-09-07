use crate::bloom::BloomFilter; // Use our own bloom filter
use bincode;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex, RwLock};

const SPARSE_INDEX_STRIDE: usize = 10;
type MemTable = BTreeMap<String, Option<String>>;

pub struct StorageEngine {
    memtable: Arc<RwLock<MemTable>>,
    sstable_files: Arc<RwLock<Vec<String>>>,
    wal: Arc<Mutex<BufWriter<File>>>,
}

impl StorageEngine {
    // ... new, set, delete, get, compact methods are unchanged ...
    pub fn new() -> io::Result<Self> {
        let wal_path = "zenith.wal";
        let wal_file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(wal_path)?;

        let mut memtable = MemTable::new();
        println!("Replaying WAL to recover state...");
        let reader = BufReader::new(&wal_file);
        for line in reader.lines() {
            let line = line?;
            if let Some((op, rest)) = line.split_once(' ') {
                if let Some((key, value)) = rest.split_once(',') {
                    match op {
                        "SET" => { memtable.insert(key.to_string(), Some(value.to_string())); },
                        "DELETE" => { memtable.insert(key.to_string(), None); },
                        _ => {}
                    }
                }
            }
        }
        println!("Recovery complete. {} items in MemTable.", memtable.len());

        let wal_writer = BufWriter::new(wal_file);
        Ok(Self {
            memtable: Arc::new(RwLock::new(memtable)),
            sstable_files: Arc::new(RwLock::new(Vec::new())),
            wal: Arc::new(Mutex::new(wal_writer)),
        })
    }

    pub fn set(&self, key: String, value: String) -> io::Result<()> {
        let mut wal_handle = self.wal.lock().unwrap();
        writeln!(wal_handle, "SET {},{}", key, &value)?;
        wal_handle.flush()?;

        let mut memtable = self.memtable.write().unwrap();
        memtable.insert(key, Some(value));

        if memtable.len() >= 5 {
            println!("MemTable full. Flushing to disk...");
            if let Err(e) = flush_memtable_and_clear_wal(&memtable, self.sstable_files.clone(), &mut wal_handle) {
                eprintln!("Failed to flush MemTable to disk: {}", e);
            }
            memtable.clear();
        }
        Ok(())
    }

    pub fn delete(&self, key: String) -> io::Result<()> {
        let mut wal_handle = self.wal.lock().unwrap();
        writeln!(wal_handle, "DELETE {},", &key)?;
        wal_handle.flush()?;

        let mut memtable = self.memtable.write().unwrap();
        memtable.insert(key, None);
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let memtable = self.memtable.read().unwrap();
        if let Some(value_opt) = memtable.get(key) {
            return value_opt.clone();
        }

        let files = self.sstable_files.read().unwrap();
        for filename in files.iter().rev() {
            if let Ok(Some(value_opt)) = search_sstable(filename, key) {
                return value_opt;
            }
        }
        None
    }

    pub fn compact(&self) -> io::Result<()> {
        println!("Starting compaction...");
        let mut files_to_compact = self.sstable_files.write().unwrap();
        if files_to_compact.len() < 2 {
            println!("Not enough files to compact.");
            return Ok(());
        }

        let mut all_data = BTreeMap::<String, Option<String>>::new();

        for filename in files_to_compact.iter() {
            let file = File::open(filename)?;
            let reader = io::BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if let Some((k, v)) = line.split_once(',') {
                    if v == "TOMBSTONE" {
                        all_data.insert(k.to_string(), None);
                    } else {
                        all_data.insert(k.to_string(), Some(v.to_string()));
                    }
                }
            }
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_filename = format!("{}.sstable-compacted", timestamp);

        let file = OpenOptions::new().create(true).write(true).open(&new_filename)?;
        let mut writer = BufWriter::new(file);

        for (key, value_opt) in all_data.iter() {
            if let Some(value) = value_opt {
                writeln!(writer, "{},{}", key, value)?;
            }
        }
        writer.flush()?;

        let old_files = files_to_compact.clone();
        *files_to_compact = vec![new_filename];

        for file in old_files {
            fs::remove_file(file)?;
        }

        println!("Compaction finished.");
        Ok(())
    }
}

fn flush_memtable_and_clear_wal(
    memtable: &MemTable,
    sstable_files: Arc<RwLock<Vec<String>>>,
    wal_handle: &mut BufWriter<File>,
) -> io::Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let filename = format!("{}.sstable", timestamp);

    let file = OpenOptions::new().create(true).write(true).open(&filename)?;
    let mut writer = BufWriter::new(file);

    let mut bloom = BloomFilter::new(10000, 0.01);
    let mut sparse_index = BTreeMap::<String, u64>::new();
    let mut current_offset: u64 = 0;
    let mut count = 0;

    for (key, value_opt) in memtable.iter() {
        bloom.add(key);
        if count % SPARSE_INDEX_STRIDE == 0 {
            sparse_index.insert(key.clone(), current_offset);
        }

        let line = match value_opt {
            Some(value) => format!("{},{}\n", key, value),
            None => format!("{},TOMBSTONE\n", key),
        };

        writer.write_all(line.as_bytes())?;
        current_offset += line.len() as u64;
        count += 1;
    }

    let bloom_offset = current_offset;
    let bloom_bytes = bincode::serialize(&bloom).unwrap();
    writer.write_all(&bloom_bytes)?;
    current_offset += bloom_bytes.len() as u64;

    let index_offset = current_offset;
    let mut index_bytes = Vec::new();
    for (key, offset) in sparse_index.iter() {
        let line = format!("{},{}\n", key, offset);
        index_bytes.extend_from_slice(line.as_bytes());
    }
    writer.write_all(&index_bytes)?;

    writer.write_all(&bloom_offset.to_le_bytes())?;
    writer.write_all(&index_offset.to_le_bytes())?;

    writer.flush()?;
    sstable_files.write().unwrap().push(filename);

    let wal_file = wal_handle.get_mut();
    wal_file.set_len(0)?;
    wal_file.seek(io::SeekFrom::Start(0))?;
    println!("WAL file cleared.");

    Ok(())
}

fn search_sstable(filename: &str, key: &str) -> io::Result<Option<Option<String>>> {
    let mut file = File::open(filename)?;

    file.seek(SeekFrom::End(-16))?;
    let mut footer_buf = [0u8; 16];
    file.read_exact(&mut footer_buf)?;
    let bloom_offset = u64::from_le_bytes(footer_buf[0..8].try_into().unwrap());
    let index_offset = u64::from_le_bytes(footer_buf[8..16].try_into().unwrap());

    file.seek(SeekFrom::Start(bloom_offset))?;
    let bloom_size = index_offset - bloom_offset;
    let mut bloom_buf = vec![0; bloom_size as usize];
    file.read_exact(&mut bloom_buf)?;

    let bloom: BloomFilter = bincode::deserialize(&bloom_buf).unwrap();

    if !bloom.contains(key) {
        return Ok(None);
    }

    file.seek(SeekFrom::Start(index_offset))?;
    let index_size = file.metadata()?.len() - index_offset - 16;
    let mut index_buf = vec![0; index_size as usize];
    file.read_exact(&mut index_buf)?;
    let index_str = String::from_utf8(index_buf).unwrap();

    let mut sparse_index = BTreeMap::<String, u64>::new();
    for line in index_str.lines() {
        if let Some((k, offset_str)) = line.split_once(',') {
            if let Ok(offset) = offset_str.parse::<u64>() {
                sparse_index.insert(k.to_string(), offset);
            }
        }
    }

    let mut block_offset = 0;
    if let Some((_, &offset)) = sparse_index.range(..=key.to_string()).next_back() {
        block_offset = offset;
    }

    file.seek(SeekFrom::Start(block_offset))?;
    let block_reader = BufReader::new(file);

    for line in block_reader.lines() {
        let line = line?;
        if let Some((k, v)) = line.split_once(',') {
            if k > key { return Ok(None); }
            if k == key {
                return Ok(Some(if v == "TOMBSTONE" { None } else { Some(v.to_string()) }));
            }
        }
    }

    Ok(None)
}