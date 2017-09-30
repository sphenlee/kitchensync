extern crate walkdir;
extern crate sha1;

use self::walkdir::{WalkDir, DirEntry, WalkDirIterator};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::btree_map::Entry;
use std::fs::File;
use std::io::Read;

use store::{Store, StoreItem};
use super::KResult;

#[derive(Debug)]
pub struct FileStat {
    name: PathBuf,
    timestamp: u64
}

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

pub fn get_files<P: AsRef<Path>>(root: P) -> KResult<Vec<FileStat>> {
    Ok(WalkDir::new(root)
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
        .collect())
}

fn get_sha1(name: &Path) -> KResult<String> {
    let mut h = sha1::Sha1::new();
    let mut fp = File::open(name)?;
    let mut buf = [0u8; 4096];

    loop {
        let read = fp.read(&mut buf)?;
        if read == 0 {
            break;
        }
        h.update(&buf[..read]);
    }

    Ok(h.digest().to_string())
}

fn added_file(filestat: &FileStat) -> KResult<StoreItem> {
    Ok(StoreItem {
        sha: get_sha1(&filestat.name)?,
        timestamp: filestat.timestamp,
        seen: true
    })
}

fn compare_file(existing: &mut StoreItem, filestat: &FileStat) -> KResult<()> {
    existing.seen = true;

    if existing.timestamp != filestat.timestamp {
        let sha = get_sha1(&filestat.name)?;
        if existing.sha != sha {
            println!("U {}", filestat.name.to_string_lossy());
            existing.sha = sha;
            existing.timestamp = filestat.timestamp;
        }
    }

    Ok(())
}

pub fn update_store(mut store: Store, files: Vec<FileStat>) -> KResult<Store> {
    for filestat in files {
        //println!("# {}", filestat.name.to_string_lossy());

        match store.files_mut().entry(filestat.name.clone()) {
            Entry::Vacant(entry) => {
                println!("A {}", entry.key().to_string_lossy());
                entry.insert(added_file(&filestat)?);
            },
            Entry::Occupied(entry) => {
                compare_file(entry.into_mut(), &filestat)?;
            }
        };
    }

    for (name, item) in store.files().iter() {
        if !item.seen {
            println!("D {}", name.to_string_lossy());
        }
    }

    Ok(store.into_iter()
        .filter(|&(ref _path, ref item)| item.seen)
        .collect::<Store>())
}
