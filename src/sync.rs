use store::{Store, StoreItem};
use remote::Remote;
use super::KResult;

use std::path::PathBuf;
use std::fs;

#[derive(Debug)]
pub enum Action {
    Add,
    Remove,
    Touch,
    Update
}

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
        Some(Action::Touch)
    } else {
        None
    }
}

pub fn get_actions(mut dest: Store, src: Store) -> Vec<(PathBuf, Action)> {
    let mut actions: Vec<_> = src.files().iter()
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
        });

    actions.extend(removes);

    actions
}

pub fn show_actions(actions: Vec<(PathBuf, Action)>) {
    for (name, action) in actions {
        println!("{} {}",
            action.get_code(),
            name.to_string_lossy());
    }
}

pub fn perform_pull_actions(actions: Vec<(PathBuf, Action)>, remote: &mut Box<Remote>) -> KResult<()> {
    for (name, action) in actions {
        println!("{} {}",
            action.get_code(),
            name.to_string_lossy());

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

    Ok(())
}