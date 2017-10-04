use store::{Store, StoreItem};
use remote::Remote;
use super::KResult;

use std::path::PathBuf;
use std::fs;

use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug)]
pub enum Action {
    Add,
    Remove,
    Touch,
    Update
}

pub type Actions = Vec<(PathBuf, Action)>;

impl Action {
    fn get_code(&self) -> char {
        match *self {
            Action::Add => 'A',
            Action::Remove => 'R',
            Action::Touch => 'T',
            Action::Update => 'U'
        }
    }
}

fn compare_items(sitem: &StoreItem, ditem: &StoreItem) -> Option<Action> {
    if sitem.sha != ditem.sha {
        Some(Action::Update)
    } else if sitem.timestamp != ditem.timestamp {
        None // decide if this is useful - Some(Action::Touch)
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

fn create_progress_bar(len: usize) -> ProgressBar {
    let progress = ProgressBar::new(len as u64);

    progress.set_style(ProgressStyle::default_bar()
        .template("{elapsed:.white} [{wide_bar:.green}] {pos:>4.white}/{len:4.white} (ETA {eta}) {msg:.cyan}")
        .progress_chars("=> "));

    progress
}

pub fn perform_pull_actions(actions: Vec<(PathBuf, Action)>, remote: &mut Box<Remote>) -> KResult<()> {
    let progress = create_progress_bar(actions.len());

    for (name, action) in actions {
        progress.inc(1);
        let msg = format!("{} {}",
            action.get_code(),
            name.to_string_lossy());
        progress.set_message(&msg);

        match action {
            Action::Add |
            Action::Update => {
                remote.get(&name, &name)?;
            },
            Action::Remove => {
                fs::remove_file(name)?;
            },
            Action::Touch => {
                // touch the file here
            }
        };
    }

    progress.finish_and_clear();

    Ok(())
}

pub fn perform_push_actions(actions: Vec<(PathBuf, Action)>, remote: &mut Box<Remote>) -> KResult<()> {
    let progress = create_progress_bar(actions.len());

    for (name, action) in actions {
        progress.inc(1);
        let msg = format!("{} {}",
                          action.get_code(),
                          name.to_string_lossy());
        progress.set_message(&msg);

        match action {
            Action::Add |
            Action::Update => {
                remote.put(&name, &name)?;
            },
            Action::Remove => {
                remote.remove(&name)?;
            },
            Action::Touch => {
                // touch the file here
            }
        };
    }

    progress.finish_and_clear();

    Ok(())
}