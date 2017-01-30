use std::io::{BufReader, BufRead, Write};
use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct StoreItem {
    pub sha: String,
    pub timestamp: u64,
    pub seen: bool
}

type StoreTuple = (PathBuf, StoreItem);
type StoreMap = BTreeMap<PathBuf, StoreItem>;

fn read_one_line(line: String) -> StoreTuple {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    let name: PathBuf = parts[2].into();
    let item = StoreItem {
        sha: parts[0].to_owned(),
        timestamp: parts[1].parse().unwrap(),
        seen: false
    };
    (name, item)
}

#[derive(Debug)]
pub struct Store(StoreMap);

impl Store {
    pub fn empty() -> Store {
        Store(StoreMap::new())
    }

    pub fn read<P: AsRef<Path>>(path: P) -> Store {
        File::open(path)
            .map(|f| {
                let f = BufReader::new(f);

                let files = f.lines()
                    .flat_map(|line| line.ok())
                    .map(read_one_line)
                    .collect();

                Store(files)
            })
            .unwrap_or_else(|_e| Store::empty())
    }

    pub fn files(&self) -> &StoreMap {
        &self.0
    }

    pub fn files_mut(&mut self) -> &mut StoreMap {
        &mut self.0
    }

    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) {
        File::create(path)
            .map(|f| {
                self.write(f)
            })
            .unwrap();
    }

    pub fn write<W: Write>(&self, mut out: W) {
        for (ref name, ref item) in self.files().iter() {
            if item.seen {
                let line = format!("{} {} {}\n",
                    item.sha,
                    item.timestamp,
                    name.to_str().unwrap());
            
                out.write_all(line.as_ref()).unwrap();
            }
        }
    }
}

impl FromIterator<StoreTuple> for Store {
    fn from_iter<I>(iter: I) -> Self
    where I: IntoIterator<Item=StoreTuple> {
        let mut store = Store::empty();
        store.0.extend(iter);
        store
    }
}

impl IntoIterator for Store {
    type Item = StoreTuple;
    type IntoIter = ::std::collections::btree_map::IntoIter<PathBuf, StoreItem>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
