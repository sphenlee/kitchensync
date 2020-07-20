use clout::debug;
use tokio::fs::{File, self};
use tokio::io;
use std::path::{Path, PathBuf};

use url::{ParseError, Url};
use utime;
use async_trait::async_trait;

use super::KResult;
use crate::s3::S3Remote;

// ________________________________________________________
// The Remote trait

#[async_trait]
pub trait Remote: Sync {
    async fn get(&mut self, name: &Path, dest: &Path) -> io::Result<()>;
    async fn put(&mut self, name: &Path, src: &Path) -> io::Result<()>;
    async fn remove(&mut self, name: &Path) -> io::Result<()>;
    async fn touch(&mut self, name: &Path, ts: i64) -> io::Result<()>;
}

pub fn from_location(location: &str) -> KResult<Box<dyn Remote>> {
    //let cwd = "file://" + env::current_dir().unwrap();
    //println!("{:?}", cwd);
    //let base = try!(Url::parse(cwd.to_str().unwrap()));

    match Url::parse(location) {
        Err(ParseError::RelativeUrlWithoutBase) => {
            // URL without a base is just a relative file path
            FileRemote::new(location.into())
        }
        Err(e) => Err(e.into()),
        Ok(url) => match url.scheme() {
            "file" => FileRemote::new(url.path()),
            "s3" => S3Remote::new(&url),
            scheme => Err(format!("unsupported URL scheme {}", scheme).into()),
        },
    }
}

// ________________________________________________________
// Implementation for local files

struct FileRemote {
    root: PathBuf,
}

impl FileRemote {
    fn new(root: &str) -> KResult<Box<dyn Remote>> {
        Ok(Box::new(FileRemote { root: root.into() }))
    }
}

#[async_trait]
impl Remote for FileRemote {
    async fn get(&mut self, name: &Path, dest: &Path) -> io::Result<()> {
        let resolved = self.root.join(name);

        debug!("get {:?} -> {:?}", resolved, dest);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut sink = File::create(dest).await?;
        let mut src = File::open(resolved).await?;

        io::copy(&mut src, &mut sink).await?;

        Ok(())
    }

    async fn put(&mut self, name: &Path, src: &Path) -> io::Result<()> {
        let resolved = self.root.join(name);

        debug!("put {:?} -> {:?}", src, resolved);

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut sink = File::create(resolved).await?;
        let mut src = File::open(src).await?;

        io::copy(&mut src, &mut sink).await?;

        Ok(())
    }

    async fn remove(&mut self, name: &Path) -> io::Result<()> {
        let resolved = self.root.join(name);

        debug!("remove {:?}", resolved);
        fs::remove_file(resolved).await
    }

    async fn touch(&mut self, path: &Path, ts: i64) -> io::Result<()> {
        utime::set_file_times(path, ts, ts)
    }
}
