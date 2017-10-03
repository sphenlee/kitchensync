use rusoto_s3::{S3, S3Client, GetObjectRequest};
use rusoto_core::{DefaultCredentialsProvider, Region};
use rusoto_core::default_tls_client;
use url::Url;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io;

use super::KResult;
use remote::Remote;

pub struct S3Remote {
    client: Box<S3>,
    bucket: String,
    prefix: PathBuf
}

impl S3Remote {
    pub fn new(url: &Url) -> KResult<Box<Remote>> {
        let client = Box::new(S3Client::new(
            default_tls_client().unwrap(),
            DefaultCredentialsProvider::new()?,
            Region::UsEast1
        ));

        let bucket = url.host_str().ok_or("S3 URL missing bucket")?;
        let prefix = Path::new(url.path()).strip_prefix("/").unwrap().to_owned();

        Ok(Box::new(S3Remote {
            client,
            bucket: bucket.to_owned(),
            prefix
        }))
    }
}

impl Remote for S3Remote {
    fn get(&mut self, name: &Path, dest: &Path) -> io::Result<()> {
        debug!("get {:?} -> {:?}", name, dest);

        // TODO don't allocate so much stuff here
        let mut req = GetObjectRequest::default();
        req.bucket = self.bucket.clone();
        req.key = self.prefix.join(name).to_string_lossy().into();

        let resp = self.client.get_object(&req).map_err(|err| {
            io::Error::new(io::ErrorKind::Other, err)
        })?;
        let mut body = resp.body.expect("no S3 body returned");

        let mut sink = File::create(dest)?;

        io::copy(&mut *body, &mut sink)?;

        Ok(())
    }

    fn put(&mut self, _name: &Path, _src: &Path) -> io::Result<()> {
        unimplemented!()
    }
}