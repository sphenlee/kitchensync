use std::io::{BufReader, BufRead, Write};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

#[derive(Debug)]
pub struct StoreItem {
    pub sha: String,
    pub timestamp: SystemTime
}
/*
impl StoreItem {
    pub fn new(sha: String, ts: SystemTime) {
        StoreItem{sha: sha, timestamp: ts}
    }
}
*/

fn timestamp_from_str(s: &str) -> SystemTime {
    let ts: u64 = s.parse().unwrap();
    UNIX_EPOCH + Duration::from_secs(ts)
}

fn timestamp_to_str(t: SystemTime) -> String {
    let dur = t.duration_since(UNIX_EPOCH).unwrap();
    dur.as_secs().to_string()
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
    pub files: HashMap<PathBuf, StoreItem>
}

impl Store {
    pub fn empty() -> Store {
        Store {
            files: HashMap::new()
        }
    }

    pub fn read<P: AsRef<Path>>(path: P) -> Store {
        File::open(path)
            .map(|f| {
                let f = BufReader::new(f);

                let files = f.lines()
                    .flat_map(|line| line.ok())
                    .map(read_one_line)
                    .collect();

                Store {
                    files: files
                }
            })
            .unwrap_or_else(|_e| Store::empty())
    }

    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) {
        File::create(path)
            .map(|f| {
                self.write(f)
            })
            .unwrap();
    }

    pub fn write<W: Write>(&self, mut out: W) {
        for (ref name, ref item) in self.files.iter() {
            let line = format!("{} {} {}\n",
                item.sha,
                timestamp_to_str(item.timestamp),
                name.to_str().unwrap());
            
            out.write_all(line.as_ref()).unwrap();
        }
    }
}
