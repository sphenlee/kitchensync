extern crate clap;

mod store;
mod update;
mod remote;
mod sync;

use store::Store;

use std::path::Path;
use std::fs;

const KSYNC: &'static str = ".ksync";
const KSYNCREMOTE: &'static str = ".ksync.remote";

fn main() {
    let args = clap::App::new("kitchensync")
        .version("0.1")
        .author("Stephen Lee <sphen.lee@gmail.com>")
        .about("Serverless file synchronisation tool")
        .setting(clap::AppSettings::SubcommandRequired)
        .subcommand(clap::SubCommand::with_name("update")
            .about("Updates the local store")
        )
        .subcommand(clap::SubCommand::with_name("sync")
            .about("Perform a synchronisation")
            .arg(clap::Arg::with_name("target")
                .help("The remote target to synchronize with")
                .required(true)
            )
            .arg(clap::Arg::with_name("push")
                .long("push")
                .short("p")
                .help("Push to the remote store rather than pulling")
            )
            .arg(clap::Arg::with_name("dry-run")
                .long("dry-run")
                .short("n")
                .help("Dry run, print actions but do not perform them")
            )
        )
        .get_matches();


    match args.subcommand() {
        ("update", Some(subargs)) => do_update(subargs),
        ("sync", Some(subargs)) => do_sync(subargs),
        _ => panic!("subcommands are supposed to be enforced by clap")
    };
}

fn do_update<'a>(args: &clap::ArgMatches<'a>) {
    println!("# updating");

    let store = Store::read(KSYNC);

    println!("# getting files");
    let files = update::get_files(".");

    println!("# looking for changes");
    let updated_store = update::update_store(store, files);

    updated_store.write_to_file(KSYNC);

    println!("# update successful");
}


fn do_sync<'a>(args: &clap::ArgMatches<'a>) {
    // get the remote ksync file locally
    let loc = args.value_of("target").unwrap();
    let mut remote = remote::from_location(loc).unwrap();

    println!("# syncing from {}", loc);

    let ksync = Path::new(KSYNC);
    let ksyncremote = Path::new(KSYNCREMOTE);

    remote.get(ksync, ksyncremote);

    // read both stores
    let lstore = Store::read(ksync);
    let rstore = Store::read(ksyncremote);

    //println!("LOCAL {:?}", lstore);
    //println!("REMOTE {:?}", rstore);

    // compare the stores
    let push = args.is_present("push");
    let actions = if push {
        sync::get_actions(rstore, lstore) // get_actions takes destination then source
    } else {
        sync::get_actions(lstore, rstore)
    };

    if args.is_present("dry-run") {
        sync::show_actions(actions);
    } else {
        sync::perform_actions(actions, &mut remote);
        
        if push {
            remote.put(ksync, ksync);
            fs::remove_file(ksyncremote).unwrap();
        } else {
            fs::rename(ksyncremote, ksync).unwrap();
        }
    }

    println!("# sync successful");
}
