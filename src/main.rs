//#[macro_use] extern crate log;

use clout::{self, error, info, status, success};

mod progress;
mod remote;
mod s3;
mod store;
mod sync;
mod update;

use store::Store;

use std::fs;
use std::io;
use std::path::Path;
use tokio;

const KSYNC: &'static str = ".kitchensync";
const KSYNCREMOTE: &'static str = ".kitchensync-remote";

type KResult<T> = Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() {
    let args = parse_args();

    let env = env_logger::Env::new()
        .filter("KSYNC_LOG")
        .write_style("KSYNC_LOG_STYLE");
    let mut builder = env_logger::Builder::from_env(env);
    builder.init();

    clout::init()
        .with_verbose(args.occurrences_of("verbose") as u8)
        .with_quiet(args.is_present("quiet"))
        .done()
        .expect("error setting up clout");

    std::process::exit(match dispatch_command(args).await {
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
        .arg(
            clap::Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .multiple(true)
                .help("Output more logging"),
        )
        .arg(
            clap::Arg::with_name("quiet")
                .long("quiet")
                .short("q")
                .help("Silence all logging"),
        )
        .subcommand(
            clap::SubCommand::with_name("update")
                .about("Updates the local store")
                .arg(
                    clap::Arg::with_name("deleted")
                        .long("deleted")
                        .help("check for deleted files"),
                ),
        )
        .subcommand(
            clap::SubCommand::with_name("sync")
                .about("Perform a synchronisation")
                .arg(
                    clap::Arg::with_name("target")
                        .help("The remote target to synchronize with")
                        .required(true),
                )
                .arg(
                    clap::Arg::with_name("push")
                        .long("push")
                        .short("p")
                        .help("Push to the remote store rather than pulling"),
                )
                .arg(
                    clap::Arg::with_name("dry-run")
                        .long("dry-run")
                        .short("n")
                        .help("Dry run, print actions but do not perform them"),
                )
                .arg(
                    clap::Arg::with_name("delete")
                        .long("delete")
                        .help("delete files removed from remote target"),
                ),
        )
        .get_matches();

    args
}

async fn dispatch_command(args: clap::ArgMatches<'_>) -> KResult<()> {
    match args.subcommand() {
        ("update", Some(subargs)) => {
            let opts = UpdateOpts {
                deleted: subargs.is_present("deleted"),
            };
            do_update(opts)
        }
        ("sync", Some(subargs)) => {
            let opts = SyncOpts {
                target: subargs.value_of("target").unwrap().to_owned(),
                push: subargs.is_present("push"),
                dry_run: subargs.is_present("dry-run"),
                delete: subargs.is_present("delete"),
            };
            do_sync(opts).await
        }
        _ => panic!("subcommands are supposed to be enforced by clap"),
    }
}

pub struct UpdateOpts {
    deleted: bool,
}

fn do_update(opts: UpdateOpts) -> KResult<()> {
    status!("updating");

    let store = Store::read(KSYNC).unwrap_or_else(|_err| Store::empty());

    info!("getting files");
    let files = update::get_files(".")?;

    info!("looking for changes");
    let updated_store = update::update_store(&opts, store, files)?;

    updated_store.write_to_file(KSYNC)?;

    success!("update successful");

    Ok(())
}

struct SyncOpts {
    target: String,
    push: bool,
    dry_run: bool,
    delete: bool,
}

async fn do_sync(opts: SyncOpts) -> KResult<()> {
    // get the remote ksync file locally
    let mut remote = remote::from_location(&opts.target)?;

    status!(
        "syncing {} {}",
        (if opts.push { "to" } else { "from" }),
        opts.target
    );

    let ksync = Path::new(KSYNC);
    let ksyncremote = Path::new(KSYNCREMOTE);

    info!("get remote store locally");
    let mut got_remote_store = true;
    remote.get(ksync, ksyncremote).await.or_else(|err| {
        if err.kind() == io::ErrorKind::NotFound && opts.push {
            got_remote_store = false;
            Ok(())
        } else {
            Err(err)
        }
    })?;

    // read both stores
    info!("reading local store");
    let lstore = Store::read(ksync).unwrap_or_else(|_err| Store::empty());
    info!("reading remote store");
    let rstore = Store::read(ksyncremote).unwrap_or_else(|_err| Store::empty());

    //println!("LOCAL {:?}", lstore);
    //println!("REMOTE {:?}", rstore);

    // compare the stores
    info!("comparing stores");
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

        info!("removing local copy of remote store");
        fs::remove_file(ksyncremote)?;
    } else {
        info!("performing sync");

        if opts.push {
            sync::perform_push_actions(actions, &mut *remote).await?;

            info!("uploading store to remote");
            remote.put(ksync, ksync).await?;

            if got_remote_store {
                info!("removing local copy of remote store");
                fs::remove_file(ksyncremote)?;
            }
        } else {
            sync::perform_pull_actions(actions, &mut *remote).await?;

            info!("update local store");
            fs::rename(ksyncremote, ksync)?;
        }
    }

    success!("sync successful");

    Ok(())
}
