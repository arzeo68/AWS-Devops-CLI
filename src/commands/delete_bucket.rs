use crate::commands::aws_utils::AwsCtx;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use aws_sdk_s3::Client;
use promkit::preset::confirm::Confirm;
use promkit::preset::listbox::Listbox;

/// Empty a bucket, including all object versions and delete markers (works for
/// both versioned and unversioned buckets). Deletes are batched (1000/request).
async fn empty_bucket(
    client: &Client,
    bucket_name: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    println!("Listing object versions to delete...");
    let mut deleted = 0usize;
    let mut key_marker: Option<String> = None;
    let mut version_marker: Option<String> = None;

    loop {
        let resp = client
            .list_object_versions()
            .bucket(bucket_name)
            .set_key_marker(key_marker.clone())
            .set_version_id_marker(version_marker.clone())
            .send()
            .await?;

        let mut ids: Vec<ObjectIdentifier> = Vec::new();
        for v in resp.versions() {
            if let Some(key) = v.key() {
                let mut builder = ObjectIdentifier::builder().key(key);
                if let Some(vid) = v.version_id() {
                    builder = builder.version_id(vid);
                }
                ids.push(builder.build()?);
            }
        }
        for marker in resp.delete_markers() {
            if let Some(key) = marker.key() {
                let mut builder = ObjectIdentifier::builder().key(key);
                if let Some(vid) = marker.version_id() {
                    builder = builder.version_id(vid);
                }
                ids.push(builder.build()?);
            }
        }

        // delete_objects accepts at most 1000 identifiers per request.
        for chunk in ids.chunks(1000) {
            let delete = Delete::builder()
                .set_objects(Some(chunk.to_vec()))
                .build()?;
            client
                .delete_objects()
                .bucket(bucket_name)
                .delete(delete)
                .send()
                .await?;
            deleted += chunk.len();
            println!("Deleted {} object versions...", deleted);
        }

        if resp.is_truncated().unwrap_or(false) {
            key_marker = resp.next_key_marker().map(|s| s.to_string());
            version_marker = resp.next_version_id_marker().map(|s| s.to_string());
        } else {
            break;
        }
    }

    Ok(deleted)
}

async fn list_buckets(client: &Client) -> Vec<String> {
    let buckets = match client.list_buckets().send().await {
        Ok(out) => out,
        Err(err) => {
            println!("Error listing buckets: {:?}", err);
            return vec![];
        }
    };
    buckets
        .buckets()
        .iter()
        .filter_map(|b| b.name().map(|n| n.to_string()))
        .collect()
}

pub async fn delete_bucket(ctx: &AwsCtx) {
    let config = ctx.config().await;
    let client = Client::new(&config);

    let buckets_names = list_buckets(&client).await;
    if buckets_names.is_empty() {
        println!("No buckets found");
        return;
    }
    let bucket = Listbox::new(&buckets_names)
        .title("Which bucket do you want to delete?")
        .listbox_lines(5)
        .prompt()
        .unwrap()
        .run()
        .unwrap();

    let confirmation_text = format!("Are you sure that you want to delete {} ?", bucket);
    let mut confirm = Confirm::new(confirmation_text).prompt().unwrap();
    let confirm_string = confirm.run();
    let confirm_string = match confirm_string {
        Ok(value) => value,
        Err(_) => {
            print!("Aborted by user");
            std::process::exit(1);
        }
    };
    drop(confirm);
    let status = confirm_string == "yes" || confirm_string == "y";
    if !status {
        println!("Aborted by user");
        std::process::exit(1);
    }

    match empty_bucket(&client, &bucket).await {
        Ok(count) => println!("Emptied bucket ({} object versions removed)", count),
        Err(err) => {
            println!("Failed to empty bucket: {:?}", err);
            return;
        }
    }

    match client.delete_bucket().bucket(&bucket).send().await {
        Ok(_) => println!("Bucket {} deleted successfully", bucket),
        Err(err) => println!("Failed to delete bucket: {:?}", err),
    }
}
