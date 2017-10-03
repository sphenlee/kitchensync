use walkdir::{WalkDir, DirEntry, WalkDirIterator};
use sha1;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::File;
use std::io::Read;

use store::{Store, StoreItem, StoreTuple};
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
            let name = entry.path().strip_prefix("./").unwrap().to_owned();
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
    println!("A {}", filestat.name.to_string_lossy());

    Ok(StoreItem {
        sha: get_sha1(&filestat.name)?,
        timestamp: filestat.timestamp,
        seen: true
    })
}

fn compare_file(existing: StoreItem, filestat: &FileStat) -> KResult<StoreItem> {
    let mut output = existing.clone();
    output.seen = true;

    if existing.timestamp != filestat.timestamp {
        trace!("timestamp changed {:?} {:?}", filestat, existing);
        let sha = get_sha1(&filestat.name)?;
        if existing.sha != sha {
            trace!("sha changed {:?} {:?}", filestat, existing);
            println!("U {}", filestat.name.to_string_lossy());
            output.sha = sha;
            output.timestamp = filestat.timestamp;
        }
    }

    Ok(output)
}

pub fn update_store(mut store: Store, files: Vec<FileStat>) -> KResult<(Store, Vec<StoreTuple>)> {
    let mut updated_store = Vec::new();

    for file in files {
        trace!("checking {:?}", file.name);
        let updated_item = match store.files_mut().remove(&file.name) {
            None => added_file(&file)?,
            Some(item) => compare_file(item, &file)?
        };

        updated_store.push((file.name, updated_item));
    }

    let deleted = store.into_iter().collect();

    Ok((updated_store.into_iter().collect(), deleted))
}

pub fn report_deleted(store: Vec<StoreTuple>) {
    for (name, _item) in store {
        println!("D {}", name.to_string_lossy());
    }
}
