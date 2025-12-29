use async_trait::async_trait;
use clout::debug;
use rusoto_s3::{
    DeleteObjectRequest, GetObjectError, GetObjectRequest, PutObjectRequest, S3Client,
    StreamingBody, S3,
};
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io;
use tokio_util::codec::{BytesCodec, Framed};
use url::Url;

use super::KResult;
use crate::remote::{self, Remote};
use futures::TryStreamExt;
use rusoto_core::{Region, RusotoError};

impl<T: std::error::Error + 'static> From<RusotoError<T>> for remote::Error {
    fn from(err: RusotoError<T>) -> Self {
        remote::Error::Other(err.to_string())
    }
}

pub struct S3Remote {
    client: S3Client,
    bucket: String,
    prefix: PathBuf,
}

impl S3Remote {
    pub fn new(url: &Url) -> KResult<Box<dyn Remote>> {
        let bucket = url.host_str().ok_or("S3 URL missing bucket")?;
        let prefix = Path::new(url.path()).strip_prefix("/").unwrap().to_owned();

        Ok(Box::new(S3Remote {
            client: S3Client::new(Region::default()),
            bucket: bucket.to_owned(),
            prefix,
        }))
    }
}

#[async_trait]
impl Remote for S3Remote {
    async fn get(&mut self, name: &Path, dest: &Path) -> remote::Result<()> {
        // TODO don't allocate so much stuff here

        let mut req = GetObjectRequest::default();
        req.bucket = self.bucket.clone();
        let key: String = self.prefix.join(name).to_string_lossy().into();
        req.key = key.clone(); // need to clone so we can report an error

        debug!("get s3://{}/{} -> {:?}", req.bucket, req.key, dest);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }

        let client = self.client.clone();
        let resp = client.get_object(req).await.map_err(|err| match err {
            RusotoError::Service(GetObjectError::NoSuchKey(_)) => {
                debug!("s3 no such key {:?}", key);
                remote::Error::NotFound(PathBuf::from(key))
            }
            err => err.into(),
        })?;
        let mut body = resp.body.expect("no S3 body returned").into_async_read();
        let mut sink = File::create(dest).await?;
        io::copy(&mut body, &mut sink).await?;
        Ok(())
    }

    async fn put(&mut self, name: &Path, src: &Path) -> remote::Result<()> {
        // TODO don't allocate so much stuff here
        let mut req = PutObjectRequest::default();
        req.bucket = self.bucket.clone();
        req.key = self.prefix.join(name).to_string_lossy().into();

        let file = File::open(src).await?;
        req.content_length = Some(file.metadata().await?.len() as i64);

        let stream = Framed::new(file, BytesCodec::new()).map_ok(|bytes| bytes.freeze());

        req.body = Some(StreamingBody::new(stream));

        debug!("put {:?} -> s3://{}/{}", src, req.bucket, req.key);

        let client = self.client.clone();
        client.put_object(req).await?;
        Ok(())
    }

    async fn remove(&mut self, name: &Path) -> remote::Result<()> {
        let mut req = DeleteObjectRequest::default();
        req.bucket = self.bucket.clone();
        req.key = self.prefix.join(name).to_string_lossy().into();

        debug!("remove s3://{}/{}", req.bucket, req.key);

        let client = self.client.clone();
        client.delete_object(req).await?;
        Ok(())
    }

    async fn touch(&mut self, _path: &Path, _ts: i64) -> remote::Result<()> {
        // can't touch files in S3
        Ok(())
    }
}
