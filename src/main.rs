//#[macro_use] extern crate log;

use anyhow::format_err;
use clout::{self, error, info, status, success};

mod config;
mod progress;
mod remote;
mod s3;
mod store;
mod sync;
mod update;

use store::Store;

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use tokio;

const KSYNC: &str = ".kitchensync";
const KSYNCREMOTE: &str = ".kitchensync-remote";
const KCONFIG: &str = ".kitchensync.toml";
const DEFAULT_TARGET: &str = "default";

type KResult<T> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> ExitCode {
    let args = parse_args();

    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_env("KSYNC_LOG"))
        .init();

    clout::init()
        .with_verbose(args.get_count("verbose") as u8)
        .with_quiet(args.get_flag("quiet"))
        .done()
        .expect("error setting up clout");

    match dispatch_command(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{}", e);
            ExitCode::FAILURE
        }
    }
}

fn parse_args() -> clap::ArgMatches {
    let args = clap::Command::new("kitchensync")
        .version("0.1")
        .author("Stephen Lee <sphen.lee@gmail.com>")
        .about("Serverless file synchronisation tool")
        .subcommand_required(true)
        .arg(
            clap::Arg::new("verbose")
                .long("verbose")
                .short('v')
                .action(clap::ArgAction::Count)
                .help("Output more logging"),
        )
        .arg(
            clap::Arg::new("quiet")
                .long("quiet")
                .short('q')
                .action(clap::ArgAction::SetTrue)
                .help("Silence all logging"),
        )
        .subcommand(
            clap::Command::new("update")
                .about("Updates the local store")
                .arg(
                    clap::Arg::new("deleted")
                        .long("deleted")
                        .action(clap::ArgAction::SetTrue)
                        .help("check for deleted files"),
                ),
        )
        .subcommand(
            clap::Command::new("sync")
                .about("Perform a synchronisation")
                .arg(clap::Arg::new("target").help("The remote target to synchronize with"))
                .arg(
                    clap::Arg::new("push")
                        .long("push")
                        .short('p')
                        .action(clap::ArgAction::SetTrue)
                        .help("Push to the remote store rather than pulling"),
                )
                .arg(
                    clap::Arg::new("dry-run")
                        .long("dry-run")
                        .short('n')
                        .action(clap::ArgAction::SetTrue)
                        .help("Dry run, print actions but do not perform them"),
                )
                .arg(
                    clap::Arg::new("delete")
                        .long("delete")
                        .action(clap::ArgAction::SetTrue)
                        .help("delete files removed from remote target"),
                ),
        )
        .get_matches();

    args
}

async fn dispatch_command(args: clap::ArgMatches) -> KResult<()> {
    let config = config::load()?;

    match args.subcommand() {
        Some(("update", subargs)) => {
            let opts = UpdateOpts {
                deleted: subargs.get_flag("deleted"),
            };
            do_update(opts)
        }
        Some(("sync", subargs)) => {
            let cli_target = subargs.get_one::<String>("target").cloned();

            let target = cli_target.or_else(|| {
                config
                    .destination
                    .iter()
                    .find(|item| item.name == DEFAULT_TARGET)
                    .map(|item| item.target.clone())
            });

            let target = target.ok_or(
                format_err!("target must be specified on the command line, or provided in the config file"),
            )?;

            let opts = SyncOpts {
                target,
                push: subargs.get_flag("push"),
                dry_run: subargs.get_flag("dry-run"),
                delete: subargs.get_flag("delete"),
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
    let mut remote = remote::from_location(&opts.target).await?;

    status!(
        "syncing {} {}",
        (if opts.push { "to" } else { "from" }),
        opts.target
    );

    let ksync = Path::new(KSYNC);
    let ksyncremote = Path::new(KSYNCREMOTE);

    info!("get remote store locally");
    let got_remote_store = remote.exists(ksync).await?;

    if got_remote_store {
        remote
        .get(ksync, ksyncremote)
        .await?;
    } else if !opts.push {
        return Err(format_err!("remote store not found, cannot perform a pull sync"));
    }

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
