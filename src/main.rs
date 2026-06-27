use clap::{command, Arg, Command};
mod commands;
use commands::aws_utils::AwsCtx;

fn delete_bucket_command() -> Command {
    Command::new("delete-bucket").about("Delete a bucket")
}

fn init_aws_state() -> Command {
    Command::new("init-aws-state").about("Init a dynamoDB and an S3 bucket")
}

fn ecs_connect_command() -> Command {
    Command::new("ecs").about("Connect or port forward to an ECS container")
}

fn ec2_connect_command() -> Command {
    Command::new("ec2").about("Connect or port forward to an EC2 container")
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

fn tf_unlock_command() -> Command {
    Command::new("tf-unlock").about("Force-remove a stuck Terraform state lock from DynamoDB")
}

#[::tokio::main]
async fn main() {
    let matches = command!() // requires `cargo` feature
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("profile")
                .long("profile")
                .global(true)
                .help("AWS profile to use (overrides AWS_PROFILE)"),
        )
        .arg(
            Arg::new("region")
                .long("region")
                .global(true)
                .help("AWS region to use (overrides AWS_REGION)"),
        )
        .subcommand(init_command())
        .subcommand(module_command())
        .subcommand(ecs_connect_command())
        .subcommand(ec2_connect_command())
        .subcommand(init_aws_state())
        .subcommand(delete_bucket_command())
        .subcommand(tf_unlock_command())
        .get_matches();

    match matches.subcommand() {
        Some(("init", _sub_matches)) => commands::init::init(),
        Some(("module", sub_matches)) => commands::module::module(sub_matches),
        Some(("ecs", sub_matches)) => commands::ecs_connect::ecs_connect(AwsCtx::from_matches(sub_matches)).await,
        Some(("ec2", sub_matches)) => commands::ec2_connect::ec2_connect(AwsCtx::from_matches(sub_matches)).await,
        Some(("init-aws-state", sub_matches)) => commands::init_aws_state::init_aws_state(&AwsCtx::from_matches(sub_matches)).await,
        Some(("delete-bucket", sub_matches)) => commands::delete_bucket::delete_bucket(&AwsCtx::from_matches(sub_matches)).await,
        Some(("tf-unlock", sub_matches)) => commands::tf_unlock::tf_unlock(&AwsCtx::from_matches(sub_matches)).await,
        _ => println!("No valid subcommand was used, please use the --help flag for more information"),
    }
}
