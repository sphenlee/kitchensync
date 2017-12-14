use store::{Store, StoreItem};
use remote::Remote;
use progress::Reporter;
use super::KResult;

use std::path::{Path, PathBuf};
use std::fs;

use utime;

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Add,
    Remove,
    Touch(u64),
    Update
}

pub type Actions = Vec<(PathBuf, Action)>;

impl Action {
    pub fn get_code(&self) -> char {
        match *self {
            Action::Add => 'A',
            Action::Remove => 'R',
            Action::Touch(_) => 'T',
            Action::Update => 'U'
        }
    }
}

fn compare_items(sitem: &StoreItem, ditem: &StoreItem) -> Option<Action> {
    if sitem.sha != ditem.sha {
        Some(Action::Update)
    } else if sitem.timestamp != ditem.timestamp {
        Some(Action::Touch(sitem.timestamp))
    } else {
        None
    }
}

pub fn get_actions(mut dest: Store, src: Store) -> (Actions, Actions) {
    let actions = src.files().iter()
        .flat_map(|(name, sitem)| {
            trace!("checking {:?}", name);
            match dest.files_mut().remove(name) {
                None => {
                    Some(Action::Add)
                },
                Some(ditem) => {
                    compare_items(&sitem, &ditem)
                }
            }.map(|action| {
                (name.clone(), action)
            })
        })
        .collect();

    let removes = dest.into_iter()
        .map(|(name, _ditem)| {
            (name, Action::Remove)
        })
        .collect();

    (actions, removes)
}

pub fn show_actions(actions: Vec<(PathBuf, Action)>) {
    for (name, action) in actions {
        println!("{} {}",
            action.get_code(),
            name.to_string_lossy());
    }
}

fn format_message(action: Action, name: &Path) -> String {
    format!("{} {}",
        action.get_code(),
        name.to_string_lossy())
}

pub fn perform_pull_actions(actions: Vec<(PathBuf, Action)>, remote: &Remote) -> KResult<()> {

    let reporter = Reporter::new(actions.len());

    for (name, action) in actions {
        reporter.inc();
        reporter.report(&format_message(action, &name));

        match action {
            Action::Add |
            Action::Update => {
                remote.get(&name, &name)?;
            },
            Action::Remove => {
                fs::remove_file(name)?;
            },
            Action::Touch(ts) => {
                utime::set_file_times(&name, ts, ts)?;
            }
        };
    }

    Ok(())
}

pub fn perform_push_actions(actions: Vec<(PathBuf, Action)>, remote: &mut Box<Remote>) -> KResult<()> {
    let reporter = Reporter::new(actions.len());

    for (name, action) in actions {
        reporter.inc();
        reporter.report(&format_message(action, &name));

        match action {
            Action::Add |
            Action::Update => {
                remote.put(&name, &name)?;
            },
            Action::Remove => {
                remote.remove(&name)?;
            },
            Action::Touch(ts) => {
                remote.touch(&name, ts)?;
            }
        };
    }

    Ok(())
}