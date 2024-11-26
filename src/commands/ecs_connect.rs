use crate::commands::aws_utils::{
    get_clusters, list_cluster_services, list_service_tasks, list_task_container,
};
use aws_sdk_ec2 as ec2;
use promkit::preset::listbox::Listbox;
use promkit::preset::readline::Readline;

pub struct AwsResource {
    pub(crate) arn: String,
    pub(crate) name: String,
}

pub struct ECSContainer {
    pub(crate) name: String,
    pub(crate) runtime_id: String,
}

async fn connect_to_ec2_command(target: &str) {
    ctrlc::set_handler(move || {}).expect("Error setting Ctrl-C handler");

    let command = format!("aws ssm start-session --target {}", target);

    println!("{}", command);

    let output = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("failed to execute process");
    let _ = output.wait_with_output();
}

pub(crate) async fn execute_command(cluster: &str, task: &str, container: &str, command: &str) {
    ctrlc::set_handler(move || {}).expect("Error setting Ctrl-C handler");
    let command = format!("aws ecs execute-command --cluster {} --task {} --container {} --command '{}' --interactive", cluster, task, container, command);
    println!("{}", command);
    let output = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("failed to execute process");
    let _ = output.wait_with_output();
}

pub async fn ecs_connect() {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_ecs::Client::new(&config);

    let clusters = get_clusters(&client).await;
    if clusters.is_empty() {
        println!("No clusters found");
        return;
    }
    let clusters_arn: Vec<String> = clusters.iter().map(|c| c.arn.clone()).collect();
    let clusters_name: Vec<String> = clusters.iter().map(|c| c.name.clone()).collect();
    let cluster = Listbox::new(&clusters_name)
        .title("What cluster do you want?")
        .listbox_lines(8)
        .prompt()
        .unwrap()
        .run()
        .unwrap();
    let cluster = &clusters_arn[crate::commands::cli_utils::get_index_of(&clusters_name, cluster)];

    let services = list_cluster_services(&client, &cluster).await;
    if services.is_empty() {
        println!("No services found");
        return;
    }
    let services_arn: Vec<String> = services.iter().map(|s| s.arn.clone()).collect();
    let services_name: Vec<String> = services.iter().map(|s| s.name.clone()).collect();
    let service = Listbox::new(&services_name)
        .title("What service do you want?")
        .listbox_lines(10)
        .prompt()
        .unwrap()
        .run()
        .unwrap();
    let service = &services_arn[crate::commands::cli_utils::get_index_of(&services_name, service)];

    let tasks = list_service_tasks(&client, &cluster, &service).await;
    if tasks.is_empty() {
        println!("No tasks found");
        return;
    }
    let tasks_arn: Vec<String> = tasks.iter().map(|t| t.arn.clone()).collect();
    let tasks_name: Vec<String> = tasks.iter().map(|t| t.name.clone()).collect();
    let task = Listbox::new(&tasks_name)
        .title("What task do you want?")
        .listbox_lines(5)
        .prompt()
        .unwrap()
        .run()
        .unwrap();
    let task = &tasks_arn[crate::commands::cli_utils::get_index_of(&tasks_name, task)];

    let containers = list_task_container(&client, &cluster, &task).await;
    if containers.is_empty() {
        println!("No containers found");
        return;
    }
    let containers_name: Vec<String> = containers.iter().map(|c| c.name.clone()).collect();
    let container = Listbox::new(&containers_name)
        .title("What container do you want?")
        .listbox_lines(5)
        .prompt()
        .unwrap()
        .run()
        .unwrap();

    let command = Readline::default()
        .title("What command do you wanna run ?")
        .validator(
            |text| text.len() > 0,
            |text| format!("You should put a valid command {}", text.len()),
        )
        .prompt()
        .unwrap()
        .run()
        .unwrap();

    execute_command(&cluster, &task, &container, &command).await;
}

async fn connect_to_ec2_instance() {
    let config = aws_config::load_from_env().await;
    let client = ec2::Client::new(&config);

    let instances = crate::commands::aws_utils::list_ec2_instances(&client).await;
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

    connect_to_ec2_command(&target).await;
}

pub async fn instance_connect() {
    let selected_type = crate::commands::cli_utils::select_type();
    match selected_type.as_str() {
        "EC2" => {
            connect_to_ec2_instance().await;
        }
        "ECS container" => {
            ecs_connect().await;
        }
        _ => {
            println!("Invalid selection");
        }
    }
}
