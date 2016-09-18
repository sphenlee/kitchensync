extern crate url;

use std::env;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use self::url::{Url, ParseError};


// ________________________________________________________
// Error handling stuff

#[derive(Debug)]
pub enum RemoteError {
    InvalidUrl(ParseError),
    UnsupportedScheme(String)
}

impl Error for RemoteError {
    fn description(&self) -> &str {
        match *self {
            RemoteError::InvalidUrl(_) => "invalid URL",
            RemoteError::UnsupportedScheme(_) => "unsupported URL scheme"
        }
    }
}

impl fmt::Display for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RemoteError::InvalidUrl(ref inner) => inner.fmt(f),
            RemoteError::UnsupportedScheme(ref scheme) => write!(f, "unsupported URL scheme {}", scheme)
        }
    }
}

impl From<ParseError> for RemoteError {
    fn from(err: ParseError) -> RemoteError {
        RemoteError::InvalidUrl(err)
    }
}

// ________________________________________________________
// The Remote trait

pub trait Remote {
    fn get(&mut self, name: &str, dest: &Path) -> io::Result<()>;
    fn put(&mut self, name: &str, src: &Path) -> io::Result<()>;
}

pub fn from_location(location: &str) -> Result<Box<Remote>, RemoteError> {
    //let cwd = "file://" + env::current_dir().unwrap();
    //println!("{:?}", cwd);
    //let base = try!(Url::parse(cwd.to_str().unwrap()));

    match Url::parse(location) {
        Err(ParseError::RelativeUrlWithoutBase) => {
            // URL without a base is just a relative file path
            Ok(FileRemote::new(location.into()))
        },
        Err(e) => {
            Err(RemoteError::InvalidUrl(e))
        }
        Ok(url) => {
            match url.scheme() {
                "file" => Ok(FileRemote::new(url.path().into())),
                //"s3" => Ok(S3Remote::new(url)),
                scheme => Err(RemoteError::UnsupportedScheme(scheme.to_owned()))
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
    fn new(root: &str) -> Box<FileRemote> {
        Box::new(FileRemote {
            root: root.into()
        })
    }
}

impl Remote for FileRemote {
    fn get(&mut self, name: &str, dest: &Path) -> io::Result<()> {
        println!("get {:?} -> {:?}", self.root.join(name), dest);

        let mut sink = try!(File::create(dest));
        let mut src = try!(File::open(self.root.join(name)));

        try!(io::copy(&mut src, &mut sink));

        Ok(())
    }

    fn put(&mut self, name: &str, src: &Path) -> io::Result<()> {
        println!("put {:?} -> {}", src, name);

        let mut sink = try!(File::create(self.root.join(name)));
        let mut src = try!(File::open(src));

        try!(io::copy(&mut src, &mut sink));

        Ok(())
    }
}
