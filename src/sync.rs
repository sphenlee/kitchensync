use store::{Store, StoreItem};
use remote::Remote;

use std::path::PathBuf;
use std::collections::HashSet;
use std::fs;

#[derive(Debug)]
pub enum Action {
    Add,
    Remove,
    Duplicate(String),
    Touch,
    Update
}

fn get_code(action: &Action) -> char {
    match *action {
        Action::Add => 'A',
        Action::Remove => 'R',
        Action::Touch => 'T',
        Action::Duplicate(_) => 'U',
        Action::Update => 'U'
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

pub fn get_actions<'a>(dest: &'a Store, src: &'a Store) -> Vec<(&'a PathBuf, Action)> {
    let mut allfiles: HashSet<&'a PathBuf> = {
        dest.files.keys().collect()
    };

    let mut actions: Vec<_> = src.files.iter()
        .flat_map(|(name, sitem)| {
            allfiles.remove(name);

            match dest.files.get(name) {
                None => {
                    Some((name, Action::Add))
                },
                Some(ref ditem) => {
                    compare_items(&sitem, &ditem).map(|action| (name, action))
                }
            }
        })
        .collect();

    actions.extend(allfiles.into_iter().map(|name| {
        (name, Action::Remove)
    }));

    actions
}

pub fn show_actions<'a>(actions: Vec<(&'a PathBuf, Action)>) {
    for (name, action) in actions {
        println!("{} {}",
            get_code(&action),
            name.to_string_lossy());
    }
}

pub fn perform_actions<'a>(actions: Vec<(&'a PathBuf, Action)>, remote: &mut Box<Remote>) {
    for (name, action) in actions {
        println!("{} {}",
            get_code(&action),
            name.to_string_lossy());

        match action {
            Action::Add |
            Action::Update => {
                remote.get(&name, &name);
            },
            Action::Remove => {
                fs::remove_file(name);
            },
            Action::Touch => {
                // touch the file here
            },
            Action::Duplicate(_) => {
                // copy the file here
            }
        };
    }    
}