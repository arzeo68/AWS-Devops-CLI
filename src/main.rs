use clap::{command, Arg, Command};
mod commands;

fn ecs_connect_command() -> Command {
    Command::new("ecs-connect")
        .about("Connect to an ECS container")
}

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
}

#[::tokio::main]
async fn main() {

    ctrlc::set_handler(move || {}).expect("Error setting Ctrl-C handler");

    let matches = command!() // requires `cargo` feature
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(init_command())
        .subcommand(module_command())
        .subcommand(ecs_connect_command())
        .get_matches();

    match matches.subcommand() {
        Some(("init", _sub_matches)) => commands::init::init(),
        Some(("module", sub_matches)) => commands::module::module(sub_matches),
        Some(("ecs-connect", _sub_matches)) => commands::ecs_connect::ecs_connect().await,
        _ => println!("No subcommand was used, please use the --help flag for more information"),
    }
}
