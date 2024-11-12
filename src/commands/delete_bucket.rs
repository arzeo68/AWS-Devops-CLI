use aws_sdk_s3::error::SdkError;
use promkit::preset::confirm::Confirm;
use promkit::preset::listbox::Listbox;
use aws_smithy_runtime_api::client::orchestrator::HttpResponse;

use aws_sdk_s3::operation::list_objects_v2::{ListObjectsV2Error, ListObjectsV2Output};


async fn empty_bucket(client: &aws_sdk_s3::Client, bucket_name: &str) {
    let objects = list_objects(client, bucket_name).await.unwrap();
    for object in objects {
        for content in object.contents.unwrap() {
            let key = content.key.unwrap();
            let _ = client.delete_object().bucket(bucket_name).key(key).send();
        }
    }
}


async fn list_objects(
    client: &aws_sdk_s3::Client,
    bucket_name: &str,
) -> Result<Vec<ListObjectsV2Output>, SdkError<ListObjectsV2Error, HttpResponse>> {
    println!("Calling 'list_objects_v2 to pull objects to delete");
    let paginator = client
        .list_objects_v2()
        .bucket(bucket_name)
        .into_paginator()
        .send();
    paginator
        .collect::<Result<Vec<ListObjectsV2Output>, SdkError<ListObjectsV2Error, HttpResponse>>>()
        .await
}

async fn list_buckets(client: &aws_sdk_s3::Client) -> Vec<String> {
    let buckets = client.list_buckets().send().await;
    let mut res: Vec<String> = Vec::new();
    for bucket in buckets.unwrap().buckets.unwrap() {
        res.push(bucket.name.unwrap());
    }
    res
}


pub async fn delete_bucket() {

    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);

    let buckets_names = list_buckets(&client).await;
    let bucket = Listbox::new(&buckets_names)
        .title("Which bucket do you want to delete?")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap();


    let confirmation_text = format!("Are you sure that you want to delete {} ?", bucket);
    let mut confirm = Confirm::new(confirmation_text).prompt().unwrap();
    let confirm_string = confirm.run();
    let confirm_string = match confirm_string {
        Ok(value) => value,
        Err(_) => { print!("Aborted by user");std::process::exit(1); }
    };
    drop(confirm);
    let status = confirm_string == "yes" || confirm_string == "y";
    if !status {
        println!("Aborted by user");
        std::process::exit(1);
    }


    println!("{:?}", bucket);
    empty_bucket(&client, &bucket).await;
}