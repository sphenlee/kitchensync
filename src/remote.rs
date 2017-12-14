use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use url::{Url, ParseError};
use utime;

use s3::S3Remote;
use super::KResult;

// ________________________________________________________
// The Remote trait

pub trait Remote {
    fn get(&self, name: &Path, dest: &Path) -> io::Result<()>;
    fn put(&self, name: &Path, src: &Path) -> io::Result<()>;
    fn remove(&self, name: &Path) -> io::Result<()>;
    fn touch(&self, name: &Path, ts: u64) -> io::Result<()>;
}

pub fn from_location(location: &str) -> KResult<Box<Remote>> {
    //let cwd = "file://" + env::current_dir().unwrap();
    //println!("{:?}", cwd);
    //let base = try!(Url::parse(cwd.to_str().unwrap()));

    match Url::parse(location) {
        Err(ParseError::RelativeUrlWithoutBase) => {
            // URL without a base is just a relative file path
            FileRemote::new(location.into())
        },
        Err(e) => {
            Err(e.into())
        }
        Ok(url) => {
            match url.scheme() {
                "file" => FileRemote::new(url.path()),
                "s3" => S3Remote::new(&url),
                scheme => Err(format!("unsupported URL scheme {}", scheme).into())
            }
        }
    }
}

// ________________________________________________________
// Implementation for local files

struct FileRemote {
    root: PathBuf
}

impl FileRemote {
    fn new(root: &str) -> KResult<Box<Remote>> {
        Ok(Box::new(FileRemote {
            root: root.into()
        }))
    }
}

impl Remote for FileRemote {
    fn get(&self, name: &Path, dest: &Path) -> io::Result<()> {
        let resolved = self.root.join(name);

        debug!("get {:?} -> {:?}", resolved, dest);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut sink = File::create(dest)?;
        let mut src = File::open(resolved)?;

        io::copy(&mut src, &mut sink)?;

        Ok(())
    }

    fn put(&self, name: &Path, src: &Path) -> io::Result<()> {
        let resolved = self.root.join(name);

        debug!("put {:?} -> {:?}", src, resolved);

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut sink = File::create(resolved)?;
        let mut src = File::open(src)?;

        io::copy(&mut src, &mut sink)?;

        Ok(())
    }

    fn remove(&self, name: &Path) -> io::Result<()> {
        let resolved = self.root.join(name);

        debug!("remove {:?}", resolved);
        fs::remove_file(resolved)
    }

    fn touch(&self, path: &Path, ts: u64) -> io::Result<()> {
        utime::set_file_times(path, ts, ts)
    }
}
