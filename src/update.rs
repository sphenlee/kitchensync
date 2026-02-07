use clout::{debug, info};
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::{DirEntry, WalkDir};

use crate::progress::Reporter;
use crate::store::{Store, StoreItem};
use crate::{KResult, UpdateOpts};

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
    let mut h = Sha1::new();
    let mut fp = File::open(name)?;
    let mut buf = vec![0; 4096 * 16];

    loop {
        let read = fp.read(&mut buf)?;
        if read == 0 {
            break;
        }
        h.update(&buf[..read]);
    }

    let result = h.finalize();
    Ok(format!("{:x}", result))
}

fn added_file(reporter: &Reporter, filestat: &FileStat) -> KResult<StoreItem> {
    reporter.report(format!("A {}", filestat.name.to_string_lossy()));

    Ok(StoreItem {
        sha: get_sha1(&filestat.name)?,
        timestamp: filestat.timestamp,
        seen: true,
    })
}

fn compare_file(
    reporter: &Reporter,
    mut item: StoreItem,
    filestat: &FileStat,
) -> KResult<StoreItem> {
    item.seen = true;

    if item.timestamp != filestat.timestamp {
        debug!("timestamp changed {:?} {:?}", filestat, item);
        item.timestamp = filestat.timestamp;

        let sha = get_sha1(&filestat.name)?;
        if item.sha != sha {
            debug!("sha changed {:?} {:?}", filestat, item);
            reporter.report(format!("U {}", filestat.name.to_string_lossy()));
            item.sha = sha;
        } else {
            reporter.report(format!("T {}", filestat.name.to_string_lossy()));
        }
    }

    Ok(item)
}

pub fn update_store(opts: &UpdateOpts, mut store: Store, files: Vec<FileStat>) -> KResult<Store> {
    let mut updated_store = Store::empty();
    let reporter = Reporter::new(files.len());

    for file in files {
        info!("checking {:?}", file.name);
        reporter.inc();

        let updated_item = match store.files_mut().remove(&file.name) {
            None => added_file(&reporter, &file)?,
            Some(item) => compare_file(&reporter, item, &file)?,
        };

        updated_store.files_mut().insert(file.name, updated_item);
    }

    if opts.deleted {
        info!("reporting deleted files");
        for (name, _item) in store.into_iter() {
            reporter.report(format!("D {}", name.to_string_lossy()));
        }
    } else {
        info!("re-add deleted files to the store");
        updated_store.files_mut().extend(store);
    }

    Ok(updated_store)
}
