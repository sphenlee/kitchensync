use store::{Store, StoreItem};
use remote::Remote;

use std::path::PathBuf;
use std::collections::{HashSet, HashMap};
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

struct SyncState<'a> {
    src: &'a Store,
    dest: &'a Store,
    allfiles: HashSet<&'a PathBuf>,
    shamap: HashMap<String, &'a PathBuf>
}

impl<'a> SyncState<'a> {
    fn new(dest: &'a Store, src: &'a Store) -> SyncState<'a> {
        let allfiles = {
            dest.files.keys().collect()
        };

        let shamap = {
            dest.files.iter().map(|(name, item)| (item.sha.clone(), name)).collect()
        };

        SyncState {
            src: src,
            dest: dest,
            allfiles: allfiles,
            shamap: shamap
        }
    }

    fn check_duplicate(&mut self, name: &PathBuf, sha: &String) -> Option<PathBuf> {
        self.shamap.get(sha).and_then(|p| {
            if *p == name {
                None
            } else {
                Some((*p).clone())
            }
        })
    }

    fn compare_items(&mut self, name: &PathBuf, sitem: &StoreItem, ditem: &StoreItem) -> Option<Action> {
        if sitem.sha != ditem.sha {
            Some(self.check_duplicate(name, &sitem.sha).map_or(
                Action::Update,
                |dupname| Action::Duplicate(dupname)
            ))
        } else if sitem.timestamp != ditem.timestamp {
            Some(Action::Touch)
        } else {
            None
        }
    }

    fn diff_stores(mut self) -> Vec<(&'a PathBuf, Action)> {
        let mut actions: Vec<_> = self.src.files.iter()
            .flat_map(|(name, sitem)| {
                self.allfiles.remove(name);

                match self.dest.files.get(name) {
                    None => {
                        Some(self.check_duplicate(&name, &sitem.sha).map_or(
                            Action::Add,
                            |name| Action::Duplicate(name)
                        ))
                    },
                    Some(ref ditem) => {
                        self.compare_items(&name, &sitem, &ditem)
                    }
                }.map(|action| (name, action))
            })
            .collect();

        actions.extend(self.allfiles.into_iter().map(|name| {
            (name, Action::Remove)
        }));

        actions
    }
}



pub fn get_actions<'a>(dest: &'a Store, src: &'a Store) -> Vec<(&'a PathBuf, Action)> {
    let state = SyncState::new(dest, src);

    state.diff_stores()
}

pub fn show_actions<'a>(actions: Vec<(&'a PathBuf, Action)>) {
    for (name, action) in actions {
        println!("{} {}",
            action.get_code(),
            name.to_string_lossy());
    }
}

pub fn perform_actions<'a>(actions: Vec<(&'a PathBuf, Action)>, remote: &mut Box<Remote>) {
    for (name, action) in actions {
        println!("{} {}",
            action.get_code(),
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
            Action::Duplicate(ref src) => {
                println!("# copying from {}", src.to_string_lossy());
                // TODO actually duplicate the local file
                remote.get(&name, &name);  
            }
        };
    }    
}