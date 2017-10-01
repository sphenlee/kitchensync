extern crate clap;
extern crate walkdir;
extern crate sha1;
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
use std::io;

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
            .long("verbose")
            .short("v")
            .multiple(true)
            .help("Output more logging"))
        .arg(clap::Arg::with_name("quiet")
            .long("quiet")
            .short("q")
            .help("Silence all logging"))

        .subcommand(clap::SubCommand::with_name("update")
            .about("Updates the local store")

            .arg(clap::Arg::with_name("deleted")
                .long("deleted")
                .help("check for deleted files")
            )
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

            .arg(clap::Arg::with_name("delete")
                .long("delete")
                .help("delete files removed from remote target")
            )
        )
        .get_matches();

    args
}

fn dispatch_command(args: clap::ArgMatches) -> KResult<()> {
    match args.subcommand() {
        ("update", Some(subargs)) => {
            let opts = UpdateOpts {
                deleted: subargs.is_present("deleted")
            };
            do_update(opts)
        },
        ("sync", Some(subargs)) => {
            let opts = SyncOpts {
                target: subargs.value_of("target").unwrap().to_owned(),
                push: subargs.is_present("push"),
                dry_run: subargs.is_present("dry-run")
            };
            do_sync(opts)
        },
        _ => panic!("subcommands are supposed to be enforced by clap")
    }
}

struct UpdateOpts {
    deleted: bool
}

fn do_update(opts: UpdateOpts) -> KResult<()> {
    info!("updating");

    let store = Store::read(KSYNC).unwrap_or_else(|_err| Store::empty());

    debug!("getting files");
    let files = update::get_files(".")?;

    debug!("looking for changes");
    let (mut updated_store, deleted) = update::update_store(store, files)?;

    if opts.deleted {
        debug!("reporting deleted files");
        update::report_deleted(deleted);
    } else {
        debug!("re-add deleted files to the store");
        updated_store.files_mut().extend(deleted);
    }

    updated_store.write_to_file(KSYNC)?;

    info!("update successful");

    Ok(())
}

struct SyncOpts {
    target: String,
    push: bool,
    dry_run: bool
}

fn do_sync(opts: SyncOpts) -> KResult<()> {
    // get the remote ksync file locally
    let mut remote = remote::from_location(&opts.target).unwrap();

    info!("syncing from {}", opts.target);

    let ksync = Path::new(KSYNC);
    let ksyncremote = Path::new(KSYNCREMOTE);

    debug!("get remote store locally");
    remote.get(ksync, ksyncremote).or_else(|err| {
        if err.kind() == io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(err)
        }
    })?;

    // read both stores
    debug!("reading local store");
    let lstore = Store::read(ksync).unwrap_or_else(|_err| Store::empty());
    debug!("reading remote store");
    let rstore = Store::read(ksyncremote).unwrap_or_else(|_err| Store::empty());

    //println!("LOCAL {:?}", lstore);
    //println!("REMOTE {:?}", rstore);

    // compare the stores
    debug!("comparing stores");
    let actions = if opts.push {
        sync::get_actions(rstore, lstore) // get_actions takes destination then source
    } else {
        sync::get_actions(lstore, rstore)
    };

    if opts.dry_run {
        sync::show_actions(actions);

        debug!("removing local copy of remote store");
        fs::remove_file(ksyncremote)?;
    } else {
        debug!("performing sync");
        sync::perform_actions(actions, &mut remote)?;
        
        if opts.push {
            debug!("uploading store to remote");
            remote.put(ksync, ksync)?;
            debug!("removing local copy of remote store");
            fs::remove_file(ksyncremote)?;
        } else {
            debug!("update local store");
            fs::rename(ksyncremote, ksync)?;
        }
    }

    info!("sync successful");

    Ok(())
}
