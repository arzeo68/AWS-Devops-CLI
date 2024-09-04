use clap::{command, Arg, ArgAction, Command};
mod commands;

fn module_command() -> Command {
    Command::new("module")
        .about("Create a new terraform module")
        .arg(
            Arg::new("name")
                .required(true),
        )
        .arg(
            Arg::new("path")
                .required(true),
        )
}

fn init_command() -> Command {
    Command::new("init")
        .about("Init a terraform repository")
        .arg(
            Arg::new("environment")
                .short('e')
                .long("environment")
                .required(true)
                .action(ArgAction::Append),
        ).arg(
        Arg::new("account")
            .short('a')
            .long("account")
            .required(true)
            .action(ArgAction::Append),
    ).arg(
        Arg::new("project")
            .short('p')
            .long("project")
            .required(true)
            .action(ArgAction::Set),
    ).arg(
        Arg::new("region")
            .short('r')
            .long("region")
            .action(ArgAction::Set),
    ).arg(
        Arg::new("path")
            .required(true),
    )
}

fn main() {
    let matches = command!() // requires `cargo` feature
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(init_command())
        .subcommand(module_command())
        .get_matches();

    match matches.subcommand() {
        Some(("init", sub_matches)) => commands::init::init(sub_matches),
        Some(("module", sub_matches)) => commands::module::module(sub_matches),
        _ => println!("No subcommand was used, please use the --help flag for more information"),
    }
}
