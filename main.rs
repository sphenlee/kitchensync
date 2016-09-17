extern crate clap;

mod store;
mod update;

use store::Store;

fn main() {
    let args = clap::App::new("kitchensync")
        .version("0.1")
        .author("Stephen Lee <sphen.lee@gmail.com>")
        .about("Serverless file synchronisation tool")
        .subcommand(clap::SubCommand::with_name("update")
            .about("Updates the local store"))
        .get_matches();


    match args.subcommand() {
        ("update", Some(subargs)) => do_update(subargs),
        _ => panic!("no subcommand!"),
    };
}

fn do_update<'a>(args: &clap::ArgMatches<'a>) {
    println!("update {:?}", args);

    let store = Store::read(".ksync");
    let files = update::get_files(".");

    println!("{:?}", store);
    println!("{:?}", files.len());
}
