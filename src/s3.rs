use anyhow::format_err;
use async_trait::async_trait;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use aws_sdk_s3_transfer_manager as s3tm;
use clout::debug;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use url::Url;

use super::KResult;
use crate::remote::{self, Remote};

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
    transfer_manager: aws_sdk_s3_transfer_manager::Client,
    s3_client: aws_sdk_s3::Client,
    bucket: String,
    prefix: PathBuf,
}

impl S3Remote {
    pub async fn new(url: &Url) -> KResult<Box<dyn Remote>> {
        let bucket = url.host_str().ok_or(format_err!("S3 URL missing bucket"))?;
        let prefix = Path::new(url.path()).strip_prefix("/").unwrap().to_owned();

        let config = aws_config::load_from_env().await;

        let tm_config = s3tm::from_env().load().await;
        let transfer_manager = s3tm::Client::new(tm_config);

        Ok(Box::new(S3Remote {
            transfer_manager,
            s3_client: aws_sdk_s3::Client::new(&config),
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

        let req = self.s3_client.head_object().bucket(&self.bucket).key(&key);

        debug!("exists s3://{}/{}", self.bucket, key);

        let resp =
            req.send()
                .await
                .map(|_| true)
                .or_else(move |err| match err.into_service_error() {
                    HeadObjectError::NotFound(_) => Ok(false),
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
            .transfer_manager
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
            .transfer_manager
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
        let key = self
            .prefix
            .join(name)
            .to_str()
            .ok_or_else(|| format_err!("non utf-8 characters in filename: {:?}", name))?
            .to_owned();

        debug!("remove s3://{}/{}", self.bucket, key);

        let req = self
            .s3_client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key);

        req.send().await?;
        Ok(())
    }

    async fn touch(&mut self, _path: &Path, _ts: i64) -> remote::Result<()> {
        // can't touch files in S3
        Ok(())
    }
}
