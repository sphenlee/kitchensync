use anyhow::format_err;
use clout::debug;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io;

use async_trait::async_trait;
use url::{ParseError, Url};

use super::KResult;
use crate::s3::S3Remote;

// // ________________________________________________________
// // The Remote trait
// #[derive(Error, Debug)]
// pub enum Error {
//     #[error("not found: {0}")]
//     NotFound(PathBuf),
//     #[error("io error: {0}")]
//     IoError(String),
//     #[error("other error: {0}")]
//     Other(String),
// }

// impl From<io::Error> for Error {
//     fn from(ioe: io::Error) -> Self {
//         Error::IoError(ioe.to_string())
//     }
// }

// impl From<&str> for Error {
//     fn from(msg: &str) -> Self {
//         Error::Other(msg.to_owned())
//     }
// }

pub type Result<T> = anyhow::Result<T>;

#[async_trait]
pub trait Remote: Sync {
    async fn exists(&mut self, name: &Path) -> Result<bool>;
    async fn get(&mut self, name: &Path, dest: &Path) -> Result<()>;
    async fn put(&mut self, name: &Path, src: &Path) -> Result<()>;
    async fn remove(&mut self, name: &Path) -> Result<()>;
    async fn touch(&mut self, name: &Path, ts: i64) -> Result<()>;
}

pub async fn from_location(location: &str) -> KResult<Box<dyn Remote>> {
    //let cwd = "file://" + env::current_dir().unwrap();
    //println!("{:?}", cwd);
    //let base = try!(Url::parse(cwd.to_str().unwrap()));

    match Url::parse(location) {
        Err(ParseError::RelativeUrlWithoutBase) => {
            // URL without a base is just a relative file path
            FileRemote::new_boxed(location)
        }
        Err(e) => Err(e.into()),
        Ok(url) => match url.scheme() {
            "file" => FileRemote::new_boxed(url.path()),
            "s3" => S3Remote::new_boxed(&url).await,
            scheme => Err(format_err!("unsupported URL scheme {}", scheme)),
        },
    }
}

// ________________________________________________________
// Implementation for local files

struct FileRemote {
    root: PathBuf,
}

impl FileRemote {
    fn new_boxed(root: &str) -> KResult<Box<dyn Remote>> {
        Ok(Box::new(FileRemote { root: root.into() }))
    }
}

#[async_trait]
impl Remote for FileRemote {
    async fn exists(&mut self, name: &Path) -> Result<bool> {
        let resolved = self.root.join(name);
        debug!("exists {:?}", resolved);

        let exists = fs::try_exists(resolved).await?;
        Ok(exists)
    }

    async fn get(&mut self, name: &Path, dest: &Path) -> Result<()> {
        let resolved = self.root.join(name);

        debug!("get {:?} -> {:?}", resolved, dest);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut src = File::open(resolved).await?;
        let mut sink = File::create(dest).await?;

        io::copy(&mut src, &mut sink).await?;

        Ok(())
    }

    async fn put(&mut self, name: &Path, src: &Path) -> Result<()> {
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

    async fn remove(&mut self, name: &Path) -> Result<()> {
        let resolved = self.root.join(name);

        debug!("remove {:?}", resolved);
        fs::remove_file(resolved).await?;
        Ok(())
    }

    async fn touch(&mut self, path: &Path, ts: i64) -> Result<()> {
        #[allow(deprecated)]
        utime::set_file_times(path, ts, ts)?;
        Ok(())
    }
}
