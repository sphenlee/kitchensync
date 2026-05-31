use super::KResult;
use crate::remote::Remote;
use crate::store::{Store, StoreItem};

use anyhow::format_err;
use clout::{error, status, trace};
use futures::stream::{self, StreamExt};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Add,
    Remove,
    Touch,
    Update,
}

pub type Actions = Vec<(PathBuf, i64, Action)>;

impl Action {
    pub fn get_code(&self) -> char {
        match *self {
            Action::Add => 'A',
            Action::Remove => 'R',
            Action::Touch => 'T',
            Action::Update => 'U',
        }
    }
}

fn compare_items(sitem: &StoreItem, ditem: &StoreItem) -> Option<Action> {
    if sitem.sha != ditem.sha {
        Some(Action::Update)
    } else if sitem.timestamp != ditem.timestamp {
        Some(Action::Touch)
    } else {
        None
    }
}

pub fn get_actions(mut dest: Store, src: Store) -> (Actions, Actions) {
    let actions = src
        .files()
        .iter()
        .flat_map(|(name, sitem)| {
            trace!("checking {:?}", name);
            match dest.files_mut().remove(name) {
                None => Some(Action::Add),
                Some(ditem) => compare_items(sitem, &ditem),
            }
            .map(|action| (name.clone(), sitem.timestamp, action))
        })
        .collect();

    let removes = dest
        .into_iter()
        .map(|(name, _ditem)| (name, 0, Action::Remove))
        .collect();

    (actions, removes)
}

pub fn show_actions(actions: Actions) {
    for (name, _ts, action) in actions {
        status!("{} {}", action.get_code(), name.to_string_lossy());
    }
}

fn status_message(action: Action, name: &Path) {
    status!("{} {}", action.get_code(), name.to_string_lossy());
}

async fn pull_get(remote: &dyn Remote, name: &Path, ts: i64) -> KResult<()> {
    remote.get(name, name).await?;

    #[allow(deprecated)]
    utime::set_file_times(name, ts, ts)?;

    Ok(())
}

pub async fn perform_pull_actions(actions: Actions, remote: &dyn Remote) -> KResult<()> {
    let failed = Arc::new(AtomicBool::new(false));

    stream::iter(actions)
        .for_each_concurrent(10, |(name, ts, action)| {
            let failed = failed.clone();
            async move {
                status_message(action, &name);

                let result = match action {
                    Action::Add | Action::Update => pull_get(remote, &name, ts).await,
                    Action::Remove => fs::remove_file(&name).map_err(Into::into),
                    Action::Touch =>
                    {
                        #[allow(deprecated)]
                        utime::set_file_times(&name, ts, ts).map_err(Into::into)
                    }
                };

                if let Err(e) = result {
                    error!("E {}: {}", name.to_string_lossy(), e);
                    failed.store(true, Ordering::SeqCst);
                }
            }
        })
        .await;

    if failed.load(Ordering::SeqCst) {
        return Err(format_err!("download unsuccessful"));
    }

    Ok(())
}

pub async fn perform_push_actions(actions: Actions, remote: &dyn Remote) -> KResult<()> {
    let failed = Arc::new(AtomicBool::new(false));

    stream::iter(actions)
        .for_each_concurrent(10, |(name, ts, action)| {
            let failed = failed.clone();
            async move {
                status_message(action, &name);

                let result = match action {
                    Action::Add | Action::Update => remote.put(&name, &name, ts).await,
                    Action::Remove => remote.remove(&name).await,
                    Action::Touch => remote.touch(&name, ts).await,
                };

                if let Err(e) = result {
                    error!("E {}: {}", name.to_string_lossy(), e);
                    failed.store(true, Ordering::SeqCst);
                }
            }
        })
        .await;

    if failed.load(Ordering::SeqCst) {
        return Err(format_err!("upload unsuccessful"));
    }

    Ok(())
}
