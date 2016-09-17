extern crate walkdir;

use self::walkdir::WalkDir;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug)]
pub struct FileStat {
    name: PathBuf,
    timestamp: SystemTime
}

pub fn get_files<P: AsRef<Path>>(root: P) -> Vec<FileStat> {
    WalkDir::new(root)
        .into_iter()
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
