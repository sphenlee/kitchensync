use store::{Store, StoreItem};
use remote::Remote;
use super::KResult;

use std::path::PathBuf;
use std::collections::BTreeMap;
use std::fs;

#[derive(Debug)]
pub enum Action {
    Add,
    Remove,
    Duplicate(PathBuf),
    Touch,
    Update
}

impl Action {
    fn get_code(&self) -> char {
        match *self {
            Action::Add => 'A',
            Action::Remove => 'R',
            Action::Touch => 'T',
            Action::Duplicate(_) => 'U',
            Action::Update => 'U'
        }
    }
}

type ShaMap = BTreeMap<String, PathBuf>;

fn check_duplicate(shamap: &ShaMap, name: &PathBuf, sha: &String) -> Option<PathBuf> {
    shamap.get(sha).and_then(|p| {
        if p == name {
            None
        } else {
            Some(p.clone())
        }
    })
}

fn compare_items(shamap: &ShaMap, name: &PathBuf, sitem: &StoreItem, ditem: &StoreItem) -> Option<Action> {
    if sitem.sha != ditem.sha {
        Some(check_duplicate(shamap, name, &sitem.sha)
            .map_or(Action::Update, |dupname| Action::Duplicate(dupname)))
    } else if sitem.timestamp != ditem.timestamp {
        Some(Action::Touch)
    } else {
        None
    }
}



pub fn get_actions(mut dest: Store, src: Store) -> Vec<(PathBuf, Action)> {
    let shamap = {
        dest.files().iter()
            .map(|(name, item)| (item.sha.clone(), name.clone()))
            .collect()
    };

    let mut actions: Vec<_> = src.files().iter()
        .flat_map(|(name, sitem)| {

            match dest.files_mut().get_mut(name) {
                None => {
                    Some(check_duplicate(&shamap, &name, &sitem.sha)
                        .map_or(Action::Add, |name| Action::Duplicate(name)))
                },
                Some(ref mut ditem) => {
                    ditem.seen = true;
                    compare_items(&shamap, &name, &sitem, &ditem)
                }
            }.map(|action| (name.clone(), action))
        })
        .collect();

    let removes = dest.files_mut().iter()
        .filter(|&(_name, ditem)| !ditem.seen)
        .map(|(name, _ditem)| (name.clone(), Action::Remove));

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

pub fn perform_actions(actions: Vec<(PathBuf, Action)>, remote: &mut Box<Remote>) -> KResult<()> {
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
            },
            Action::Duplicate(ref src) => {
                info!("copying from {}", src.to_string_lossy());
                // TODO actually duplicate the local file
                remote.get(&name, &name)?;
            }
        };
    }

    Ok(())
}