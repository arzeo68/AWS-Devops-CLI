use aws_sdk_ecs as ecs;
use promkit::preset::listbox::Listbox;
use promkit::preset::readline::Readline;
use aws_sdk_ec2 as ec2;


pub struct AwsResource {
    pub(crate) arn: String,
    pub(crate) name: String,
}

pub struct ECSContainer {
    pub(crate) name: String,
    pub(crate) runtime_id: String,
}

pub(crate) fn get_index_of<T: PartialEq>(vec: &Vec<T>, value: T) -> usize {
    vec.iter().position(|x| *x == value).unwrap()
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

pub(crate) async fn list_task_container(client: &ecs::Client, cluster: &str, task: &str) -> Vec<ECSContainer> {
    let mut res: Vec<ECSContainer> = Vec::new();
    let containers = client.describe_tasks().cluster(cluster).tasks(task.to_string()).send().await;
    if containers.is_err() {
        println!("Error listing containers: {:?}", containers.err());
        return vec![];
    }

    for container in containers.unwrap().tasks.unwrap().clone() {
        for container in container.containers.unwrap().clone() {
            let container_name = container.name.clone().unwrap();
            res.push(ECSContainer { name: container_name, runtime_id: container.runtime_id.unwrap() });
        }
    }
    res
}

pub(crate) async fn list_service_tasks(client: &ecs::Client, cluster: &str, service: &str) -> Vec<AwsResource> {
    let mut res: Vec<AwsResource> = Vec::new();
    let tasks = client.list_tasks().cluster(cluster).service_name(service).send().await;
    if tasks.is_err() {
        println!("Error listing tasks: {:?}", tasks.err());
        return vec![];
    }

    for task in tasks.unwrap().task_arns.unwrap().clone() {
        let task_name = task.split("/").last().unwrap().to_string();
        res.push(AwsResource { arn: task, name: task_name });
    }

    res
}

pub(crate) async fn list_cluster_services(client: &ecs::Client, cluster: &str) -> Vec<AwsResource> {
    let mut res: Vec<AwsResource> = Vec::new();
    let services = client.list_services().cluster(cluster).send().await;
    if services.is_err() {
        println!("Error listing services: {:?}", services.err());
        return vec![];
    }
    for service in services.unwrap().service_arns.unwrap().clone() {
        let service_name = service.split("/").last().unwrap().to_string();
        res.push(AwsResource { arn: service, name: service_name });
    }

    res
}

pub(crate) async fn get_clusters(client: &ecs::Client) -> Vec<AwsResource> {
    let mut res: Vec<AwsResource> = Vec::new();
    let clusters = client.list_clusters().send().await;
    if clusters.is_err() {
        println!("Error listing clusters: {:?}", clusters.err());
        return vec![];
    }
    for cluster in clusters.unwrap().cluster_arns.unwrap().clone() {
        let cluster_name = cluster.split("/").last().unwrap().to_string();
        res.push(AwsResource { arn: cluster, name: cluster_name });
    }
    res
}

pub async fn ecs_connect() {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_ecs::Client::new(&config);

    let clusters = get_clusters(&client).await;
    if clusters.is_empty() { println!("No clusters found"); return; }
    let clusters_arn: Vec<String> = clusters.iter().map(|c| c.arn.clone()).collect();
    let clusters_name: Vec<String> = clusters.iter().map(|c| c.name.clone()).collect();
    let cluster = Listbox::new(&clusters_name)
        .title("What cluster do you want?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();
    let cluster = &clusters_arn[get_index_of(&clusters_name, cluster)];


    let services = list_cluster_services(&client, &cluster).await;
    if services.is_empty() { println!("No services found"); return; }
    let services_arn: Vec<String> = services.iter().map(|s| s.arn.clone()).collect();
    let services_name: Vec<String> = services.iter().map(|s| s.name.clone()).collect();
    let service = Listbox::new(&services_name)
        .title("What service do you want?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();
    let service = &services_arn[get_index_of(&services_name, service)];

    let tasks = list_service_tasks(&client, &cluster, &service).await;
    if tasks.is_empty() { println!("No tasks found"); return; }
    let tasks_arn: Vec<String> = tasks.iter().map(|t| t.arn.clone()).collect();
    let tasks_name: Vec<String> = tasks.iter().map(|t| t.name.clone()).collect();
    let task = Listbox::new(&tasks_name)
        .title("What task do you want?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();
    let task = &tasks_arn[get_index_of(&tasks_name, task)];


    let containers = list_task_container(&client, &cluster, &task).await;
    if containers.is_empty() { println!("No containers found"); return; }
    let containers_name: Vec<String> = containers.iter().map(|c| c.name.clone()).collect();
    let container = Listbox::new(&containers_name)
        .title("What container do you want?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();

    let command = Readline::default()
        .title("What command do you wanna run ?")
        .validator(
            |text| text.len() > 0,
            |text| format!("You should put a valid command {}", text.len()),
        )
        .prompt().unwrap().run().unwrap();

    execute_command(&cluster, &task, &container, &command).await;
}

async fn connect_to_ec2_instance() {

    let config = aws_config::load_from_env().await;
    let client = ec2::Client::new(&config);

    let instances = crate::commands::port_forward::list_ec2_instances(&client).await;
    if instances.is_empty() { println!("No instances found"); return; }
    let instances_id: Vec<String> = instances.iter().map(|i| i.instance_id.clone()).collect();
    let instances_name: Vec<String> = instances.iter().map(|i| i.name.clone()).collect();
    let instance = Listbox::new(&instances_name)
        .title("Which instance do you want?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();
    let target = &instances_id[crate::commands::ecs_connect::get_index_of(&instances_name, instance)];

    connect_to_ec2_command(&target).await;
}

fn select_type() -> String {
    let types = vec!["EC2", "ECS container"];
    Listbox::new(&types)
        .title("Select the type of resource you want to port forward")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap()
}
pub async fn instance_connect() {
    let selected_type = select_type();
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