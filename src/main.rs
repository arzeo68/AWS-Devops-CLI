use clap::{command, Arg, Command};
mod commands;

fn delete_bucket_command() -> Command {
    Command::new("delete-bucket").about("Delete a bucket")
}

fn init_aws_state() -> Command {
    Command::new("init-aws-state").about("Init a dynamoDB and an S3 bucket")
}

fn ecs_connect_command() -> Command {
    Command::new("connect").about("Connect to an instance (EC2 / ECS)")
}

fn port_forward() -> Command {
    Command::new("port-forward").about("Forward a port from a container/EC2 to your local machine")
}

fn module_command() -> Command {
    Command::new("module")
        .about("Create a new terraform module")
        .arg(Arg::new("name").required(true))
        .arg(Arg::new("path").required(true))
}

fn init_command() -> Command {
    Command::new("init").about("Init a terraform repository")
}

#[::tokio::main]
async fn main() {
    let matches = command!() // requires `cargo` feature
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(init_command())
        .subcommand(module_command())
        .subcommand(ecs_connect_command())
        .subcommand(init_aws_state())
        .subcommand(port_forward())
        .subcommand(delete_bucket_command())
        .get_matches();

    match matches.subcommand() {
        Some(("init", _sub_matches)) => commands::init::init(),
        Some(("module", sub_matches)) => commands::module::module(sub_matches),
        Some(("connect", _sub_matches)) => commands::ecs_connect::instance_connect().await,
        Some(("init-aws-state", _sub_matches)) => commands::inti_aws_state::init_aws_state().await,
        Some(("port-forward", _sub_matches)) => commands::port_forward::port_forward().await,
        Some(("delete-bucket", _sub_matches)) => commands::delete_bucket::delete_bucket().await,
        _ => println!("No subcommand was used, please use the --help flag for more information"),
    }
}
