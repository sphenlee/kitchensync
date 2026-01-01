use anyhow::format_err;
use async_trait::async_trait;
use aws_sdk_s3_transfer_manager as s3tm;
use clout::debug;
use rusoto_core::request::BufferedHttpResponse;
use rusoto_s3::{DeleteObjectRequest, HeadObjectError, HeadObjectRequest, S3, S3Client};
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use url::Url;

use super::KResult;
use crate::remote::{self, Remote};
use rusoto_core::{Region, RusotoError};

// impl<T: std::error::Error + 'static> From<RusotoError<T>> for remote::Error {
//     fn from(err: RusotoError<T>) -> Self {
//         remote::Error::Other(err.to_string())
//     }
// }

// impl<T: std::error::Error + 'static> From<s3tm::error::Error> for remote::Error {
//     fn from(err: s3tm::error::Error) -> Self {
//         remote::Error::Other(err.to_string())
//     }
// }

pub struct S3Remote {
    client: aws_sdk_s3_transfer_manager::Client,
    raw_client: S3Client,
    bucket: String,
    prefix: PathBuf,
}

impl S3Remote {
    pub async fn new(url: &Url) -> KResult<Box<dyn Remote>> {
        let bucket = url.host_str().ok_or(format_err!("S3 URL missing bucket"))?;
        let prefix = Path::new(url.path()).strip_prefix("/").unwrap().to_owned();

        let config = s3tm::from_env().load().await;
        let client = s3tm::Client::new(config);

        Ok(Box::new(S3Remote {
            client,
            raw_client: S3Client::new(Region::default()),
            bucket: bucket.to_owned(),
            prefix,
        }))
    }
}

#[async_trait]
impl Remote for S3Remote {
    async fn exists(&mut self, name: &Path) -> remote::Result<bool> {
        let key = self
            .prefix
            .join(name)
            .to_str()
            .ok_or_else(|| format_err!("non utf-8 characters in filename: {:?}", name))?
            .to_owned();

        let mut req = HeadObjectRequest::default();
        req.bucket = self.bucket.clone();
        req.key = key.clone();

        debug!("exists s3://{}/{}", self.bucket, req.key);

        let raw_client = self.raw_client.clone();
        let resp = raw_client
            .head_object(req)
            .await
            .map(|_| true)
            .or_else(move |err| match err {
                RusotoError::Service(HeadObjectError::NoSuchKey(_)) |
                RusotoError::Unknown(BufferedHttpResponse{ status: http::status::StatusCode::NOT_FOUND, ..}) => {
                    debug!("s3 no such key {:?}", key);
                    Ok(false)
                }
                err => Err(err),
            })?;

        Ok(resp)
    }

    async fn get(&mut self, name: &Path, dest: &Path) -> remote::Result<()> {
        let key = self
            .prefix
            .join(name)
            .to_str()
            .ok_or_else(|| format_err!("non utf-8 characters in filename: {:?}", name))?
            .to_owned();

        debug!("get s3://{}/{} -> {:?}", self.bucket, key, dest);

        let mut handle = self
            .client
            .download()
            .bucket(&self.bucket)
            .key(key)
            .initiate()?;

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut sink = File::create(dest).await?;

        let body = handle.body_mut();
        while let Some(chunk) = body.next().await {
            let chunk = chunk?;
            for segment in chunk.data.into_segments() {
                sink.write_all(segment.as_ref()).await?;
            }
        }

        Ok(())
    }

    async fn put(&mut self, name: &Path, src: &Path) -> remote::Result<()> {
        let key = self
            .prefix
            .join(name)
            .to_str()
            .ok_or_else(|| format_err!("non utf-8 characters in filename: {:?}", name))?
            .to_owned();

        debug!("put {:?} -> s3://{}/{}", src, self.bucket, key);

        let stream = s3tm::io::InputStream::from_path(src)?;
        let _response = self
            .client
            .upload()
            .bucket(&self.bucket)
            .key(key)
            .body(stream)
            .initiate()?
            .join()
            .await?;

        Ok(())
    }

    async fn remove(&mut self, name: &Path) -> remote::Result<()> {
        let mut req = DeleteObjectRequest::default();
        req.bucket = self.bucket.clone();
        req.key = self
            .prefix
            .join(name)
            .to_str()
            .ok_or_else(|| format_err!("non utf-8 characters in filename: {:?}", name))?
            .to_owned();

        debug!("remove s3://{}/{}", req.bucket, req.key);

        let raw_client = self.raw_client.clone();
        raw_client.delete_object(req).await?;
        Ok(())
    }

    async fn touch(&mut self, _path: &Path, _ts: i64) -> remote::Result<()> {
        // can't touch files in S3
        Ok(())
    }
}
