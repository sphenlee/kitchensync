use clout::{debug, info, status};
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::{DirEntry, WalkDir};

use crate::store::{Store, StoreItem, StoreTuple};
use crate::{KResult, UpdateOpts};
use rayon::prelude::*;

#[derive(Debug)]
pub struct FileStat {
    name: PathBuf,
    timestamp: i64,
}

fn systemtime_to_i64(t: SystemTime) -> i64 {
    let dur = t.duration_since(UNIX_EPOCH).unwrap();
    dur.as_secs() as i64
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
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
                name,
                timestamp: systemtime_to_i64(meta.modified().unwrap()),
            }
        })
        .collect())
}

fn get_sha1(name: &Path) -> KResult<String> {
    let mut hasher = Sha1::new();
    let mut reader = File::open(name)?;
    let mut buffer = [0u8; 8192];

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(result.iter().map(|byte| format!("{:02x}", byte)).collect())
}

fn added_file(filestat: FileStat) -> KResult<StoreTuple> {
    status!("A {}", filestat.name.to_string_lossy());

    let sha = get_sha1(&filestat.name)?;

    Ok((
        filestat.name,
        StoreItem {
            sha,
            timestamp: filestat.timestamp,
            seen: true,
        },
    ))
}

fn compare_file(mut item: StoreItem, filestat: FileStat) -> KResult<StoreTuple> {
    item.seen = true;

    if item.timestamp != filestat.timestamp {
        debug!("timestamp changed {:?} {:?}", filestat, item);
        item.timestamp = filestat.timestamp;

        let sha = get_sha1(&filestat.name)?;
        if item.sha != sha {
            debug!("sha changed {:?} {:?}", filestat, item);
            status!("U {}", filestat.name.to_string_lossy());
            item.sha = sha;
        } else {
            status!("T {}", filestat.name.to_string_lossy());
        }
    }

    Ok((filestat.name, item))
}

pub fn update_store(opts: &UpdateOpts, mut store: Store, files: Vec<FileStat>) -> KResult<Store> {
    let mut updated_store = Store::empty();

    let mut matched_pairs: Vec<(FileStat, Option<StoreItem>)> = Vec::with_capacity(files.len());
    for file in files {
        let found = store.files_mut().remove(&file.name);
        matched_pairs.push((file, found));
    }

    let processed: Vec<KResult<(PathBuf, StoreItem)>> = matched_pairs
        .into_par_iter()
        .map(|(file, maybe_item)| {
            info!("checking {:?}", file.name);
            match maybe_item {
                None => added_file(file),
                Some(item) => compare_file(item, file),
            }
        })
        .collect();

    for r in processed {
        match r {
            Ok((name, item)) => {
                updated_store.files_mut().insert(name, item);
            }
            Err(e) => return Err(e),
        }
    }

    if opts.deleted {
        info!("reporting deleted files");
        for (name, _item) in store.into_iter() {
            status!("D {}", name.to_string_lossy());
        }
    } else {
        info!("re-add deleted files to the store");
        updated_store.files_mut().extend(store);
    }

    Ok(updated_store)
}
