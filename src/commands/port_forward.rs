use crate::commands::aws_utils::list_ec2_instances;
use aws_sdk_ec2 as ec2;
use promkit::preset::listbox::Listbox;
use promkit::preset::readline::Readline;

pub(crate) async fn connect_to_ecs_command(target: &str, host: &str, local_port: &str, remote_port: &str) {
    ctrlc::set_handler(move || {}).expect("Error setting Ctrl-C handler");

    let document = "AWS-StartPortForwardingSessionToRemoteHost";
    let params = format!(
        "\'{{\"portNumber\":[\"{}\"],\"localPortNumber\":[\"{}\"], \"host\":[\"{}\"]}}\'",
        remote_port, local_port, host
    );
    let command = format!(
        "aws ssm start-session --target {} --document-name {} --parameters {}",
        target, document, params
    );

    println!("{}", command);

    let output = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("failed to execute process");
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

async fn connect_to_ec2_instance() {
    let config = aws_config::load_from_env().await;
    let client = ec2::Client::new(&config);

    let instances = list_ec2_instances(&client).await;
    if instances.is_empty() {
        println!("No instances found");
        return;
    }
    let instances_id: Vec<String> = instances.iter().map(|i| i.instance_id.clone()).collect();
    let instances_name: Vec<String> = instances.iter().map(|i| i.name.clone()).collect();
    let instance = Listbox::new(&instances_name)
        .title("Which instance do you want?")
        .listbox_lines(5)
        .prompt()
        .unwrap()
        .run()
        .unwrap();
    let target = &instances_id[crate::commands::cli_utils::get_index_of(&instances_name, instance)];

    let host = select_host(&"What host do you want to use?".to_string());
    let remote_port = select_port(&"What remote port do you want to use?".to_string());
    let local_port = select_port(&"What local port do you want to use?".to_string());

    connect_to_ecs_command(&target, &host, &local_port, &remote_port).await;
}

pub async fn port_forward() {
    let selected_type = crate::commands::cli_utils::select_type();
    match selected_type.as_str() {
        "EC2" => {
            connect_to_ec2_instance().await;
        }
        _ => {
            println!("Invalid selection");
        }
    }
}
