use promkit::preset::listbox::Listbox;
use promkit::preset::readline::Readline;
use aws_sdk_ec2 as ec2;

#[derive(Debug)]
pub struct EC2Instance {
    pub(crate) instance_id: String,
    pub(crate) name: String
}

async fn connect_to_ecs_command(target: &str, host: &str, local_port: &str, remote_port: &str) {
    ctrlc::set_handler(move || {}).expect("Error setting Ctrl-C handler");

    let document = "AWS-StartPortForwardingSessionToRemoteHost";
    let params = format!("\'{{\"portNumber\":[\"{}\"],\"localPortNumber\":[\"{}\"], \"host\":[\"{}\"]}}\'", remote_port, local_port, host);
    let command = format!("aws ssm start-session --target {} --document-name {} --parameters {}", target, document, params);

    println!("{}", command);

    let output = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("failed to execute process");
    let _ = output.wait_with_output();
}

fn select_port(question: &String) -> String {
    let mut port = Readline::default()
        .title(question)
        .validator(
            |text| text.parse::<f64>().is_ok() ,
            |text| format!("Your port should be a number {}", text),
        )
        .prompt().unwrap();
    let port_string = port.run();
    let port_string = match port_string {
        Ok(value) => value,
        Err(_) => { print!("Aborted by user");std::process::exit(1); }
    };
    drop(port);
    port_string
}

fn select_host(question: &String) -> String {
    let mut host = Readline::default()
        .title(question)
        .validator(
            |text| text.len() > 0,
            |text| format!("Your host can't be empty {}", text.len()),
        )
        .prompt().unwrap();
    let host_string = host.run();
    let host_string = match host_string {
        Ok(value) => value,
        Err(_) => { print!("Aborted by user");std::process::exit(1); }
    };
    host_string
}

pub(crate) async fn list_ec2_instances(client: &ec2::Client) -> Vec<EC2Instance> {
    let mut res: Vec<EC2Instance> = Vec::new();
    let instances = client.describe_instances().send().await;
    if instances.is_err() {
        println!("Error listing instances: {:?}", instances.err());
        return vec![];
    }

    for reservation in instances.unwrap().reservations.unwrap().clone() {
        for instance in reservation.instances.unwrap().clone() {
            if instance.state.unwrap().name.unwrap().as_str() != "running" { continue; }

            let instance_id = instance.instance_id.clone().unwrap();
            let name = instance.tags.unwrap().iter().find(|tag| tag.key.as_deref() == Some("Name")).unwrap().value.clone().unwrap();
            let display_name = format!("{} ({})", name, instance_id);
            res.push(EC2Instance { instance_id, name: display_name });
        }
    }
    res
}

async fn connect_to_ec2_instance() {

    let config = aws_config::load_from_env().await;
    let client = ec2::Client::new(&config);

    let instances = list_ec2_instances(&client).await;
    if instances.is_empty() { println!("No instances found"); return; }
    let instances_id: Vec<String> = instances.iter().map(|i| i.instance_id.clone()).collect();
    let instances_name: Vec<String> = instances.iter().map(|i| i.name.clone()).collect();
    let instance = Listbox::new(&instances_name)
        .title("Which instance do you want?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();
    let target = &instances_id[crate::commands::ecs_connect::get_index_of(&instances_name, instance)];


    let host = select_host(&"What host do you want to use?".to_string());
    let remote_port = select_port(&"What remote port do you want to use?".to_string());
    let local_port = select_port(&"What local port do you want to use?".to_string());

    connect_to_ecs_command(&target, &host, &local_port, &remote_port).await;
}

async fn connect_to_ecs_container() {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_ecs::Client::new(&config);

    let clusters = crate::commands::ecs_connect::get_clusters(&client).await;
    if clusters.is_empty() { println!("No clusters found"); return; }
    let clusters_name: Vec<String> = clusters.iter().map(|c| c.name.clone()).collect();
    let cluster = Listbox::new(&clusters_name)
        .title("What cluster do you want?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();


    let services = crate::commands::ecs_connect::list_cluster_services(&client, &cluster).await;
    if services.is_empty() { println!("No services found"); return; }
    let services_arn: Vec<String> = services.iter().map(|s| s.arn.clone()).collect();
    let services_name: Vec<String> = services.iter().map(|s| s.name.clone()).collect();
    let service = Listbox::new(&services_name)
        .title("What sercice do you want?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();
    let service = &services_arn[crate::commands::ecs_connect::get_index_of(&services_name, service)];

    let tasks = crate::commands::ecs_connect::list_service_tasks(&client, &cluster, &service).await;
    if tasks.is_empty() { println!("No tasks found"); return; }
    let tasks_arn: Vec<String> = tasks.iter().map(|t| t.arn.clone()).collect();
    let tasks_name: Vec<String> = tasks.iter().map(|t| t.name.clone()).collect();
    let task = Listbox::new(&tasks_name)
        .title("What task do you want?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();
    let task = &tasks_arn[crate::commands::ecs_connect::get_index_of(&tasks_name, task)];
    let task_id = task.split("/").last().unwrap();


    let containers = crate::commands::ecs_connect::list_task_container(&client, &cluster, &task).await;
    if containers.is_empty() { println!("No containers found"); return; }
    let containers_name: Vec<String> = containers.iter().map(|c| c.name.clone()).collect();
    let container = Listbox::new(&containers_name)
        .title("What container do you want?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();
    let runtime_id = containers[crate::commands::ecs_connect::get_index_of(&containers_name, container)].runtime_id.clone();

    let host = select_host(&"What host do you want to use?".to_string());
    let remote_port = select_port(&"What remote port do you want to use?".to_string());
    let local_port = select_port(&"What local port do you want to use?".to_string());

    let target = format!("ecs:{}_{}_{}", cluster, task_id, runtime_id);

    connect_to_ecs_command(&target, &host, &local_port, &remote_port).await;
}


fn select_type() -> String {
    let types = vec!["EC2", "ECS container"];
    Listbox::new(&types)
        .title("Select the type of resource you want to port forward")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap()
}

pub async fn port_forward() {
    let selected_type = select_type();
    match selected_type.as_str() {
        "EC2" => {
            connect_to_ec2_instance().await;
        }
        "ECS container" => {
            connect_to_ecs_container().await;
        }
        _ => {
            println!("Invalid selection");
        }
    }
}