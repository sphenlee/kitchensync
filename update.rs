extern crate walkdir;

use self::walkdir::{WalkDir, DirEntry, WalkDirIterator};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::collections::HashSet;
use std::collections::hash_map::Entry;

use store::{Store, StoreItem};

#[derive(Debug)]
pub struct FileStat {
    name: PathBuf,
    timestamp: SystemTime
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
        .map(|entry| {
            let name = entry.path().into();
            let meta = entry.metadata().unwrap();
            FileStat {
                name: name,
                timestamp: meta.modified().unwrap()
            }
        })
        .collect()
}

fn added_file(filestat: &FileStat) -> StoreItem {
    // compute a SHA1 here
    StoreItem {
        sha: "sha1".to_owned(),
        timestamp: filestat.timestamp
    }
}

fn compare_file(existing: &StoreItem, filestat: &FileStat) -> Option<StoreItem> {
    if existing.timestamp == filestat.timestamp {
        None
    } else {
        // grab a sha here and compare them
        Some(StoreItem {
            sha: "sha1".to_owned(),
            timestamp: filestat.timestamp
        })
    }
}

pub fn update_store(store: &mut Store, files: &Vec<FileStat>) {
    let mut allfiles: HashSet<PathBuf> = {
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
