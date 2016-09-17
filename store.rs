use std::io::{BufReader, BufRead};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

#[derive(Debug)]
pub struct StoreItem {
    sha: String,
    timestamp: SystemTime
}

fn timestamp_from_str(s: &str) -> SystemTime {
    let ts: u64 = s.parse().unwrap();
    UNIX_EPOCH + Duration::from_secs(ts)
}

fn read_one_line(line: String) -> (PathBuf, StoreItem) {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    let name: PathBuf = parts[2].into();
    let item = StoreItem {
        sha: parts[0].to_owned(),
        timestamp: timestamp_from_str(parts[1])
    };
    (name, item)
}

#[derive(Debug)]
pub struct Store {
    files: HashMap<PathBuf, StoreItem>
}

impl Store {
    pub fn empty() -> Store {
        Store {
            files: HashMap::new()
        }
    }

    pub fn read<P: AsRef<Path>>(path: P) -> Store {
        match File::open(path) {
            Err(_) => Store::empty(),
            Ok(f) => {
                let f = BufReader::new(f);

                let files = f.lines()
                    .flat_map(|line| line.ok())
                    .map(read_one_line)
                    .collect();

                Store {
                    files: files
                }
            }
        }
    }
}
