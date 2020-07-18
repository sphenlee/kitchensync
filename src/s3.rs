use clout::{debug, warn};
use rusoto_s3::{DeleteObjectRequest, GetObjectRequest, PutObjectRequest, S3Client, StreamingBody, S3, GetObjectError};
use std::fs;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use url::Url;

use super::KResult;
use crate::remote::Remote;
use rusoto_core::{Region, RusotoError};

pub struct S3Remote {
    client: S3Client,
    rt: tokio::runtime::Runtime,
    bucket: String,
    prefix: PathBuf,
}

impl S3Remote {
    pub fn new(url: &Url) -> KResult<Box<dyn Remote>> {
        let bucket = url.host_str().ok_or("S3 URL missing bucket")?;
        let prefix = Path::new(url.path()).strip_prefix("/").unwrap().to_owned();

        Ok(Box::new(S3Remote {
            client: S3Client::new(Region::default()),
            rt: tokio::runtime::Runtime::new()?,
            bucket: bucket.to_owned(),
            prefix,
        }))
    }
}

impl Remote for S3Remote {
    fn get(&mut self, name: &Path, dest: &Path) -> io::Result<()> {
        // TODO don't allocate so much stuff here

        let mut req = GetObjectRequest::default();
        req.bucket = self.bucket.clone();
        let key: String = self.prefix.join(name).to_string_lossy().into();
        req.key = key.clone(); // need to clone so we can report an error

        debug!("get s3://{}/{} -> {:?}", req.bucket, req.key, dest);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let client = self.client.clone();
        self.rt.block_on(async {
            let resp = client.get_object(req).await.map_err(|err| {
                match err {
                    RusotoError::Service(GetObjectError::NoSuchKey(_)) => {
                        debug!("s3 no such key {:?}", key);
                        io::Error::new(io::ErrorKind::NotFound, key)
                    },
                    _ => {
                        warn!("s3 get error {:?}", err);
                        io::Error::new(io::ErrorKind::NotFound, err)
                    }
                }
            })?;
            let mut body = resp.body.expect("no S3 body returned").into_async_read();
            let mut sink = tokio::fs::File::create(dest).await?;
            tokio::io::copy(&mut body, &mut sink).await?;
            Ok(())
        })
    }

    fn put(&mut self, name: &Path, src: &Path) -> io::Result<()> {
        // TODO don't allocate so much stuff here
        let mut req = PutObjectRequest::default();
        req.bucket = self.bucket.clone();
        req.key = self.prefix.join(name).to_string_lossy().into();

        let mut body = Vec::new();
        File::open(src)?.read_to_end(&mut body)?;

        req.body = Some(StreamingBody::from(body));

        debug!("put {:?} -> s3://{}/{}", src, req.bucket, req.key);

        let client = self.client.clone();
        self.rt.block_on(async {
            client.put_object(req).await.map_err(|err| {
                warn!("s3 put error: {:?}", err);
                io::Error::new(io::ErrorKind::Other, err)
            })?;
            Ok(())
        })
    }

    fn remove(&mut self, name: &Path) -> io::Result<()> {
        let mut req = DeleteObjectRequest::default();
        req.bucket = self.bucket.clone();
        req.key = self.prefix.join(name).to_string_lossy().into();

        debug!("remove s3://{}/{}", req.bucket, req.key);

        let client = self.client.clone();
        self.rt.block_on(async {
            client.delete_object(req).await.map_err(|err| {
                warn!("s3 rm error: {:?}", err);
                io::Error::new(io::ErrorKind::Other, err)
            })?;
            Ok(())
        })
    }

    fn touch(&mut self, _path: &Path, _ts: i64) -> io::Result<()> {
        // can't touch files in S3
        Ok(())
    }
}
