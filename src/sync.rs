use store::{Store, StoreItem};

use std::path::PathBuf;

#[derive(Debug)]
pub enum Action {
    Add,
    Remove,
    Duplicate(String),
    Touch,
    Update
}

fn get_code(action: Action) -> char {
    match action {
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

pub fn get_actions<'a>(dest: &Store, src: &'a Store) -> Vec<(&'a PathBuf, Action)> {
    src.files.iter()
        .flat_map(|(name, sitem)| {
            match dest.files.get(name) {
                None => {
                    Some((name, Action::Add))
                },
                Some(ref ditem) => {
                    compare_items(&sitem, &ditem).map(|action| (name, action))
                }
            }
        })
        .collect()
}

pub fn show_actions<'a>(actions: Vec<(&'a PathBuf, Action)>) {
    for (name, action) in actions {
        println!("{} {}",
            get_code(action),
            name.to_string_lossy());
    }
}

pub fn perform_actions<'a>(actions: Vec<(&'a PathBuf, Action)>) {
    
}