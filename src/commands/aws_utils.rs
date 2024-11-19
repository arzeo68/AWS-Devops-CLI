use aws_sdk_ecs as ecs;
use aws_sdk_ec2 as ec2;
use crate::commands::ecs_connect::{AwsResource, ECSContainer};

#[derive(Debug)]
pub struct EC2Instance {
    pub(crate) instance_id: String,
    pub(crate) name: String
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