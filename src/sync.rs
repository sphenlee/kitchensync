use super::KResult;
use crate::progress::Reporter;
use crate::remote::Remote;
use crate::store::{Store, StoreItem};

use clout::{status, trace};
use std::fs;
use std::path::{Path, PathBuf};

use utime;

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
                Some(ditem) => compare_items(&sitem, &ditem),
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

fn format_message(action: Action, name: &Path) -> String {
    format!("{} {}", action.get_code(), name.to_string_lossy())
}

pub async fn perform_pull_actions(actions: Actions, remote: &mut dyn Remote) -> KResult<()> {
    let reporter = Reporter::new(actions.len());

    for (name, ts, action) in actions {
        reporter.inc();
        reporter.report(&format_message(action, &name));

        match action {
            Action::Add | Action::Update => {
                remote.get(&name, &name).await?;
                utime::set_file_times(&name, ts, ts)?;
            }
            Action::Remove => {
                fs::remove_file(name)?;
            }
            Action::Touch => {
                utime::set_file_times(&name, ts, ts)?;
            }
        };
    }

    Ok(())
}

pub async fn perform_push_actions(actions: Actions, remote: &mut dyn Remote) -> KResult<()> {
    let reporter = Reporter::new(actions.len());

    for (name, ts, action) in actions {
        reporter.inc();
        reporter.report(&format_message(action, &name));

        match action {
            Action::Add | Action::Update => {
                remote.put(&name, &name).await?;
                remote.touch(&name, ts).await?;
            }
            Action::Remove => {
                remote.remove(&name).await?;
            }
            Action::Touch => {
                remote.touch(&name, ts).await?;
            }
        };
    }

    Ok(())
}
