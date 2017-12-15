use walkdir::{WalkDir, DirEntry, WalkDirIterator};
use sha1;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::File;
use std::io::Read;

use store::{Store, StoreItem};
use progress::Reporter;
use super::{KResult, UpdateOpts};

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
                name,
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

fn added_file(reporter: &Reporter, filestat: &FileStat) -> KResult<StoreItem> {
    reporter.report(format!("A {}", filestat.name.to_string_lossy()));

    Ok(StoreItem {
        sha: get_sha1(&filestat.name)?,
        timestamp: filestat.timestamp,
        seen: true
    })
}

fn compare_file(reporter: &Reporter, existing: StoreItem, filestat: &FileStat) -> KResult<StoreItem> {
    let mut output = existing.clone();
    output.seen = true;

    if existing.timestamp != filestat.timestamp {
        trace!("timestamp changed {:?} {:?}", filestat, existing);
        output.timestamp = filestat.timestamp;

        let sha = get_sha1(&filestat.name)?;
        if existing.sha != sha {
            trace!("sha changed {:?} {:?}", filestat, existing);
            reporter.report(format!("U {}", filestat.name.to_string_lossy()));
            output.sha = sha;
        } else {
            reporter.report(format!("T {}", filestat.name.to_string_lossy()));
        }
    }

    Ok(output)
}

pub fn update_store(opts: &UpdateOpts, mut store: Store, files: Vec<FileStat>) -> KResult<Store>
{
    let mut updated_store = Vec::new();
    let reporter = Reporter::new(files.len());

    for file in files {
        trace!("checking {:?}", file.name);
        reporter.inc();

        let updated_item = match store.files_mut().remove(&file.name) {
            None => added_file(&reporter, &file)?,
            Some(item) => compare_file(&reporter, item, &file)?
        };

        updated_store.push((file.name, updated_item));
    }

    if opts.deleted {
        debug!("reporting deleted files");
        for (name, _item) in store.into_iter() {
            reporter.report(format!("D {}", name.to_string_lossy()));
        }
    } else {
        debug!("re-add deleted files to the store");
        updated_store.extend(store.into_iter());
    }

    Ok(updated_store.into_iter().collect())
}
