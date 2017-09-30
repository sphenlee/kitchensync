extern crate clap;
#[macro_use] extern crate log;
extern crate colored;
extern crate atty;

mod store;
mod update;
mod remote;
mod sync;
mod logging;

use store::Store;

use std::path::Path;
use std::fs;

const KSYNC: &'static str = ".kitchensync";
const KSYNCREMOTE: &'static str = ".kitchensync-remote";

type KResult<T> = Result<T, Box<std::error::Error>>;

fn main() {
    let args = parse_args();

    let level = if args.is_present("quiet") {
        log::LogLevelFilter::Off
    } else {
        match args.occurrences_of("verbose") {
            1 => log::LogLevelFilter::Debug,
            2 => log::LogLevelFilter::Trace,
            _ => log::LogLevelFilter::Info
        }
    };
    logging::init_with_level(level).unwrap();

    std::process::exit(match dispatch_command(args) {
        Ok(()) => 0,
        Err(e) => {
            error!("{}", e);
            1
        }
    });
}

fn parse_args() -> clap::ArgMatches<'static> {
    let args = clap::App::new("kitchensync")
        .version("0.1")
        .author("Stephen Lee <sphen.lee@gmail.com>")
        .about("Serverless file synchronisation tool")
        .setting(clap::AppSettings::SubcommandRequired)

        .arg(clap::Arg::with_name("verbose")
            .short("v")
            .multiple(true)
            .help("Output more logging"))
        .arg(clap::Arg::with_name("quiet")
            .short("q")
            .help("Silence all logging"))

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

    args
}

fn dispatch_command(args: clap::ArgMatches) -> KResult<()> {
    match args.subcommand() {
        ("update", Some(subargs)) => do_update(subargs),
        ("sync", Some(subargs)) => do_sync(subargs),
        _ => panic!("subcommands are supposed to be enforced by clap")
    }
}

fn do_update<'a>(args: &clap::ArgMatches<'a>) -> KResult<()> {
    info!("updating");

    let store = Store::read(KSYNC).unwrap_or_else(|_err| Store::empty());

    debug!("getting files");
    let files = update::get_files(".")?;

    debug!("looking for changes");
    let updated_store = update::update_store(store, files)?;

    updated_store.write_to_file(KSYNC)?;

    info!("update successful");

    Ok(())
}


fn do_sync<'a>(args: &clap::ArgMatches<'a>) -> KResult<()> {
    // get the remote ksync file locally
    let loc = args.value_of("target").unwrap();
    let mut remote = remote::from_location(loc).unwrap();

    info!("syncing from {}", loc);

    let ksync = Path::new(KSYNC);
    let ksyncremote = Path::new(KSYNCREMOTE);

    remote.get(ksync, ksyncremote)?;

    // read both stores
    let lstore = Store::read(ksync).unwrap_or_else(|_err| Store::empty());
    let rstore = Store::read(ksyncremote)?;

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
        sync::perform_actions(actions, &mut remote)?;
        
        if push {
            remote.put(ksync, ksync)?;
            fs::remove_file(ksyncremote).unwrap();
        } else {
            fs::rename(ksyncremote, ksync).unwrap();
        }
    }

    info!("sync successful");

    Ok(())
}
