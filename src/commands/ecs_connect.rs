use aws_sdk_ecs as ecs;
use promkit::preset::listbox::Listbox;
use promkit::preset::readline::Readline;

struct AwsResource {
    arn: String,
    name: String,
}

fn get_index_of<T: PartialEq>(vec: &Vec<T>, value: T) -> usize {
    vec.iter().position(|x| *x == value).unwrap()
}

async fn execute_command(cluster: &str, task: &str, container: &str, command: &str) {
    let output = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("aws ecs execute-command --cluster {} --task {} --container {} --command '{}' --interactive", cluster, task, container, command))
        .spawn()
        .expect("failed to execute process");
    let _ = output.wait_with_output();
}

async fn list_task_container(client: &ecs::Client, cluster: &str, task: &str) -> Vec<AwsResource> {
    let mut res: Vec<AwsResource> = Vec::new();
    let containers = client.describe_tasks().cluster(cluster).tasks(task.to_string()).send().await;
    if containers.is_err() {
        println!("Error listing containers: {:?}", containers.err());
        return vec![];
    }
    for container in containers.unwrap().tasks.unwrap().clone() {
        for container in container.containers.unwrap().clone() {
            let container_name = container.name.clone().unwrap();
            res.push(AwsResource { arn: container.container_arn.unwrap(), name: container_name });
        }
    }
    res
}

async fn list_service_tasks(client: &ecs::Client, cluster: &str, service: &str) -> Vec<AwsResource> {
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

async fn list_cluster_services(client: &ecs::Client, cluster: &str) -> Vec<AwsResource> {
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

async fn get_clusters(client: &ecs::Client) -> Vec<AwsResource> { //TODO: handle pagination
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
        .title("What sercice do you want?")
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
