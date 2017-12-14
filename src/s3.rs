use rusoto_s3::{S3,
                S3Client,
                GetObjectRequest,
                PutObjectRequest,
                DeleteObjectRequest};
use rusoto_core::DefaultCredentialsProvider;
use rusoto_core::{default_tls_client, default_region};
use url::Url;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{self, Read};
use std::fs;

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
            default_region()
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
    fn get(&self, name: &Path, dest: &Path) -> io::Result<()> {
        // TODO don't allocate so much stuff here
        let mut req = GetObjectRequest::default();
        req.bucket = self.bucket.clone();
        req.key = self.prefix.join(name).to_string_lossy().into();

        debug!("get s3://{}/{} -> {:?}", req.bucket, req.key, dest);

        let resp = self.client.get_object(&req).map_err(|err| {
            debug!("s3 error {:?}", err);
            //if let GetObjectError::NoSuchKey(_) = err {
                io::Error::new(io::ErrorKind::NotFound, err)
            //} else {
            //    io::Error::new(io::ErrorKind::Other, err)
            //}
        })?;
        let mut body = resp.body.expect("no S3 body returned");

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut sink = File::create(dest)?;

        io::copy(&mut *body, &mut sink)?;

        Ok(())
    }

    fn put(&self, name: &Path, src: &Path) -> io::Result<()> {
        // TODO don't allocate so much stuff here
        let mut req = PutObjectRequest::default();
        req.bucket = self.bucket.clone();
        req.key = self.prefix.join(name).to_string_lossy().into();

        let mut body = Vec::new();
        File::open(src)?.read_to_end(&mut body)?;
        req.body = Some(body);

        debug!("put {:?} -> s3://{}/{}", src, req.bucket, req.key);

        self.client.put_object(&req).map_err(|err| {
            io::Error::new(io::ErrorKind::Other, err)
        })?;

        Ok(())
    }

    fn remove(&self, name: &Path) -> io::Result<()> {
        let mut req = DeleteObjectRequest::default();
        req.bucket = self.bucket.clone();
        req.key = self.prefix.join(name).to_string_lossy().into();

        debug!("remove s3://{}/{}", req.bucket, req.key);

        self.client.delete_object(&req).map_err(|err| {
            io::Error::new(io::ErrorKind::Other, err)
        })?;

        Ok(())
    }

    fn touch(&self, _path: &Path, _ts: u64) -> io::Result<()> {
        // can't touch files in S3
        Ok(())
    }
}
