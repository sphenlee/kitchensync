extern crate clap;
extern crate walkdir;
extern crate sha1;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate colored;
extern crate atty;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate url;
extern crate rayon;
extern crate indicatif;
extern crate utime;

mod store;
mod update;
mod remote;
mod sync;
//mod logging;
mod s3;
mod progress;

use store::Store;

use std::path::Path;
use std::fs;
use std::io;
use std::env;

const KSYNC: &'static str = ".kitchensync";
const KSYNCREMOTE: &'static str = ".kitchensync-remote";

type KResult<T> = Result<T, Box<std::error::Error>>;

fn main() {
    let args = parse_args();

    let level = if args.is_present("quiet") {
        log::LevelFilter::Off
    } else {
        match args.occurrences_of("verbose") {
            0 => log::LevelFilter::Info,
            1 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace
        }
    };

    let mut builder = env_logger::Builder::new();
    builder.filter(None, level);
    if let Ok(var) = env::var("KSYNC_LOG") {
        builder.parse(&var);
    }
    builder.init();

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
                dry_run: subargs.is_present("dry-run"),
                delete: subargs.is_present("delete")
            };
            do_sync(opts)
        },
        _ => panic!("subcommands are supposed to be enforced by clap")
    }
}

pub struct UpdateOpts {
    deleted: bool
}

fn do_update(opts: UpdateOpts) -> KResult<()> {
    info!("updating");

    let store = Store::read(KSYNC).unwrap_or_else(|_err| Store::empty());

    debug!("getting files");
    let files = update::get_files(".")?;

    debug!("looking for changes");
    let updated_store = update::update_store(&opts, store, files)?;

    updated_store.write_to_file(KSYNC)?;

    info!("update successful");

    Ok(())
}

struct SyncOpts {
    target: String,
    push: bool,
    dry_run: bool,
    delete: bool
}

fn do_sync(opts: SyncOpts) -> KResult<()> {
    // get the remote ksync file locally
    let mut remote = remote::from_location(&opts.target)?;

    info!("syncing {} {}",
          (if opts.push { "to" } else { "from" }),
          opts.target);

    let ksync = Path::new(KSYNC);
    let ksyncremote = Path::new(KSYNCREMOTE);

    debug!("get remote store locally");
    let mut got_remote_store = true;
    remote.get(ksync, ksyncremote).or_else(|err| {
        if err.kind() == io::ErrorKind::NotFound && opts.push {
            got_remote_store = false;
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
    let (mut actions, removes) = if opts.push {
        sync::get_actions(rstore, lstore) // get_actions takes destination then source
    } else {
        sync::get_actions(lstore, rstore)
    };

    if opts.delete {
        actions.extend(removes);
    }

    if opts.dry_run {
        sync::show_actions(actions);

        debug!("removing local copy of remote store");
        fs::remove_file(ksyncremote)?;
    } else {
        debug!("performing sync");

        if opts.push {
            sync::perform_push_actions(actions, &mut remote)?;

            debug!("uploading store to remote");
            remote.put(ksync, ksync)?;

            if got_remote_store {
                debug!("removing local copy of remote store");
                fs::remove_file(ksyncremote)?;
            }
        } else {
            sync::perform_pull_actions(actions, &mut *remote)?;

            debug!("update local store");
            fs::rename(ksyncremote, ksync)?;
        }
    }

    info!("sync successful");

    Ok(())
}
