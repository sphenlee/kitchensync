extern crate walkdir;
extern crate sha1;

use self::walkdir::{WalkDir, DirEntry, WalkDirIterator};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::BTreeSet;
use std::collections::btree_map::Entry;
use std::fs::File;
use std::io::Read;

use store::{Store, StoreItem};

#[derive(Debug)]
pub struct FileStat {
    name: PathBuf,
    timestamp: u64
}

/*fn timestamp_from_str(s: &str) -> SystemTime {
    let ts: u64 = s.parse().unwrap();
    UNIX_EPOCH + Duration::from_secs(ts)
}*/

fn systemtime_to_u64(t: SystemTime) -> u64 {
    let dur = t.duration_since(UNIX_EPOCH).unwrap();
    dur.as_secs()
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.len() > 1 && s.starts_with("."))
         .unwrap_or(false)
}

pub fn get_files<P: AsRef<Path>>(root: P) -> Vec<FileStat> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            let name = entry.path().into();
            let meta = entry.metadata().unwrap();
            FileStat {
                name: name,
                timestamp: systemtime_to_u64(meta.modified().unwrap())
            }
        })
        .collect()
}

fn get_sha1(name: &Path) -> String {
    let mut h = sha1::Sha1::new();
    let mut fp = File::open(name).unwrap();
    let mut buf = [0u8; 4096];

    loop {
        let read = fp.read(&mut buf).unwrap();
        if read == 0 {
            break;
        }
        h.update(&buf[..read]);
    }

    h.digest().to_string()
}

fn added_file(filestat: &FileStat) -> StoreItem {
    StoreItem {
        sha: get_sha1(&filestat.name),
        timestamp: filestat.timestamp
    }
}

fn compare_file(existing: &StoreItem, filestat: &FileStat) -> Option<StoreItem> {
    if existing.timestamp == filestat.timestamp {
        None
    } else {
        let sha = get_sha1(&filestat.name);
        if existing.sha == sha {
            None
        } else {
            Some(StoreItem {
                sha: sha,
                timestamp: filestat.timestamp
            })
        }
    }
}

pub fn update_store(store: &mut Store, files: &Vec<FileStat>) {
    let mut allfiles: BTreeSet<PathBuf> = {
        store.files.keys().cloned().collect()
    };

    for filestat in files {
        allfiles.remove(&filestat.name);

        //println!("# {}", filestat.name.to_string_lossy());

        match store.files.entry(filestat.name.clone()) {
            Entry::Vacant(entry) => {
                println!("A {}", filestat.name.to_string_lossy());
                entry.insert(added_file(&filestat));
            },
            Entry::Occupied(mut entry) => {
                let newitem = {
                    compare_file(&entry.get(), &filestat)
                };
                if let Some(item) = newitem {
                    println!("U {}", filestat.name.to_string_lossy());
                    entry.insert(item);
                };
            }
        };
    }

    for path in allfiles {
        println!("D {}", path.to_string_lossy());
        store.files.remove(&path);
    }
}
