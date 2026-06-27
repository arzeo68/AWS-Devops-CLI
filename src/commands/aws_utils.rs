use crate::commands::ecs_connect::{AwsResource, ECSContainer};
use aws_sdk_ec2 as ec2;
use aws_sdk_ecs as ecs;
use aws_sdk_ecs::operation::describe_clusters::{DescribeClustersOutput};
use aws_sdk_ecs::types::Service;

/// Shared AWS context derived from the global `--profile` / `--region` flags.
/// Threaded into both the SDK config and any spawned `aws` CLI subprocess.
#[derive(Clone, Default, Debug)]
pub struct AwsCtx {
    pub profile: Option<String>,
    pub region: Option<String>,
}

impl AwsCtx {
    pub fn from_matches(matches: &clap::ArgMatches) -> Self {
        Self {
            profile: matches.get_one::<String>("profile").cloned(),
            region: matches.get_one::<String>("region").cloned(),
        }
    }

    /// Build an SDK config honoring the selected profile/region, falling back to
    /// the standard environment/credential chain when unset.
    pub async fn config(&self) -> aws_config::SdkConfig {
        let mut loader = aws_config::from_env();
        if let Some(profile) = &self.profile {
            loader = loader.profile_name(profile);
        }
        if let Some(region) = &self.region {
            loader = loader.region(aws_config::Region::new(region.clone()));
        }
        loader.load().await
    }

    /// Inject profile/region into a spawned `aws` CLI process so the subprocess
    /// targets the same account/region as the SDK calls.
    pub fn apply_env(&self, cmd: &mut std::process::Command) {
        if let Some(profile) = &self.profile {
            cmd.env("AWS_PROFILE", profile);
        }
        if let Some(region) = &self.region {
            cmd.env("AWS_REGION", region);
        }
    }
}

#[derive(Debug)]
pub struct EC2Instance {
    pub(crate) instance_id: String,
    pub(crate) name: String,
}

pub(crate) async fn ecs_execute_command(ctx: &AwsCtx, cluster: &str, task: &str, container: &str, command: &str) {
    ctrlc::set_handler(move || {}).expect("Error setting Ctrl-C handler");
    // Pass arguments directly (no shell) so cluster/task/container names can't
    // be interpreted by /bin/sh.
    let mut cmd = std::process::Command::new("aws");
    cmd.args([
        "ecs",
        "execute-command",
        "--cluster",
        cluster,
        "--task",
        task,
        "--container",
        container,
        "--command",
        command,
        "--interactive",
    ]);
    ctx.apply_env(&mut cmd);
    let output = cmd.spawn().expect("failed to execute process");
    let _ = output.wait_with_output();
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
            if instance.state.unwrap().name.unwrap().as_str() != "running" {
                continue;
            }

            let instance_id = instance.instance_id.clone().unwrap();
            let name = instance
                .tags
                .unwrap()
                .iter()
                .find(|tag| tag.key.as_deref() == Some("Name"))
                .unwrap()
                .value
                .clone()
                .unwrap();
            let display_name = format!("{} ({})", name, instance_id);
            res.push(EC2Instance {
                instance_id,
                name: display_name,
            });
        }
    }
    res
}

/// Force a new deployment of a service (rolling restart of its tasks).
pub(crate) async fn force_new_deployment(client: &ecs::Client, cluster: &str, service: &str) -> bool {
    let resp = client
        .update_service()
        .cluster(cluster)
        .service(service)
        .force_new_deployment(true)
        .send()
        .await;
    match resp {
        Ok(_) => true,
        Err(err) => {
            println!("Error restarting service: {:?}", err);
            false
        }
    }
}

/// Update a service's desired task count (scale up/down).
pub(crate) async fn set_service_desired_count(
    client: &ecs::Client,
    cluster: &str,
    service: &str,
    count: i32,
) -> bool {
    let resp = client
        .update_service()
        .cluster(cluster)
        .service(service)
        .desired_count(count)
        .send()
        .await;
    match resp {
        Ok(_) => true,
        Err(err) => {
            println!("Error scaling service: {:?}", err);
            false
        }
    }
}

pub(crate) async fn list_task_container(
    client: &ecs::Client,
    cluster: &str,
    task: &str,
) -> Vec<ECSContainer> {
    let mut res: Vec<ECSContainer> = Vec::new();
    let containers = client
        .describe_tasks()
        .cluster(cluster)
        .tasks(task.to_string())
        .send()
        .await;
    if containers.is_err() {
        println!("Error listing containers: {:?}", containers.err());
        return vec![];
    }

    for container in containers.unwrap().tasks.unwrap().clone() {
        for container in container.containers.unwrap().clone() {
            if container.runtime_id == None { continue }
            let container_name = container.name.clone().unwrap();
            res.push(ECSContainer {
                name: container_name,
                runtime_id: container.runtime_id.unwrap(),
            });
        }
    }
    res
}

pub(crate) async fn list_service_tasks(
    client: &ecs::Client,
    cluster: &str,
    service: &str,
) -> Vec<AwsResource> {
    let mut res: Vec<AwsResource> = Vec::new();
    let tasks = client
        .list_tasks()
        .cluster(cluster)
        .service_name(service)
        .send()
        .await;

    if tasks.is_err() {
        println!("Error listing tasks: {:?}", tasks.err());
        return vec![];
    }


    for task in tasks.unwrap().task_arns.unwrap().clone() {
        let task_name = task.split("/").last().unwrap().to_string();
        res.push(AwsResource {
            name: task_name,
        });
    }

    res
}

pub(crate) async fn list_cluster_services(client: &ecs::Client, cluster: &str) -> Vec<Service> {
    let mut res: Vec<Service> = Vec::new();
    let services = client.list_services().cluster(cluster).max_results(10).send().await;
    if services.is_err() {
        println!("Error listing services: {:?}", services.err());
        return vec![];
    }

    let services = services.unwrap();
    let describe_services = client.describe_services().cluster(cluster).set_services(services.service_arns).send().await;
    if describe_services.is_err() {
        println!("Error describing services: {:?}", describe_services.err());
        return vec![];
    }
    describe_services.unwrap().services.unwrap().iter().for_each(|s| res.push(s.clone()));


    if services.next_token.is_some() {
        let mut next_token = services.next_token.clone().unwrap();
        loop {
            let services = client
                .list_services()
                .cluster(cluster)
                .next_token(next_token)
                .max_results(10)
                .send()
                .await;
            if services.is_err() {
                println!("Error listing services: {:?}", services.err());
                return vec![];
            }

            let services = services.unwrap();
            let describe_services = client.describe_services().cluster(cluster).set_services(services.service_arns).send().await;
            if describe_services.is_err() {
                println!("Error describing services: {:?}", describe_services.err());
                return vec![];
            }
            describe_services.unwrap().services.unwrap().iter().for_each(|s| res.push(s.clone()));

            if services.next_token.is_some() {
                next_token = services.next_token.clone().unwrap();
            } else {
                break;
            }
        }
    }

    res
}

pub(crate) async fn get_clusters(client: &ecs::Client) -> Option<DescribeClustersOutput> {
    let clusters = client.list_clusters().send().await;
    if clusters.is_err() {
        println!("Error listing clusters: {:?}", clusters.err());
        return None;
    }
    let cluster_data = client.describe_clusters().set_clusters(clusters.unwrap().cluster_arns).send().await;
    if cluster_data.is_err() {
        println!("Error describing clusters: {:?}", cluster_data.err());
        return None;
    }
    Some(cluster_data.unwrap())
}
