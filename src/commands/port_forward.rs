use crate::commands::aws_utils::AwsCtx;
use promkit::preset::readline::Readline;

pub(crate) async fn connect_to_ecs_command(ctx: &AwsCtx, target: &str, host: &str, local_port: &str, remote_port: &str) {
    ctrlc::set_handler(move || {}).expect("Error setting Ctrl-C handler");

    let document = "AWS-StartPortForwardingSessionToRemoteHost";
    // No surrounding shell quotes: the JSON is passed as a single argv element.
    let params = format!(
        "{{\"portNumber\":[\"{}\"],\"localPortNumber\":[\"{}\"],\"host\":[\"{}\"]}}",
        remote_port, local_port, host
    );

    let mut cmd = std::process::Command::new("aws");
    cmd.args([
        "ssm",
        "start-session",
        "--target",
        target,
        "--document-name",
        document,
        "--parameters",
        &params,
    ]);
    ctx.apply_env(&mut cmd);
    let output = cmd.spawn().expect("failed to execute process");
    let _ = output.wait_with_output();
}

pub(crate) fn select_port(question: &String) -> String {
    let mut port = Readline::default()
        .title(question)
        .validator(
            |text| text.parse::<f64>().is_ok(),
            |text| format!("Your port should be a number {}", text),
        )
        .prompt()
        .unwrap();
    let port_string = port.run();
    let port_string = match port_string {
        Ok(value) => value,
        Err(_) => {
            print!("Aborted by user");
            std::process::exit(1);
        }
    };
    drop(port);
    port_string
}

pub(crate) fn select_host(question: &String) -> String {
    let mut host = Readline::default()
        .title(question)
        .validator(
            |text| text.len() > 0,
            |text| format!("Your host can't be empty {}", text.len()),
        )
        .prompt()
        .unwrap();
    let host_string = host.run();
    let host_string = match host_string {
        Ok(value) => value,
        Err(_) => {
            print!("Aborted by user");
            std::process::exit(1);
        }
    };
    host_string
}
