use std::io::{BufReader, BufRead, Write};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct StoreItem {
    pub sha: String,
    pub timestamp: u64
}

fn read_one_line(line: String) -> (PathBuf, StoreItem) {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    let name: PathBuf = parts[2].into();
    let item = StoreItem {
        sha: parts[0].to_owned(),
        timestamp: parts[1].parse().unwrap()
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
                item.timestamp,
                name.to_str().unwrap());
            
            out.write_all(line.as_ref()).unwrap();
        }
    }
}
