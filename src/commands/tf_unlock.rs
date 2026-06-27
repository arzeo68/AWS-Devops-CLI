use crate::commands::aws_utils::AwsCtx;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use promkit::preset::confirm::Confirm;
use promkit::preset::listbox::Listbox;

/// List all DynamoDB tables (paginated) so the user can pick the Terraform lock table.
async fn list_tables(client: &Client) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut start: Option<String> = None;
    loop {
        let resp = match client
            .list_tables()
            .set_exclusive_start_table_name(start.clone())
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                println!("Error listing tables: {:?}", err);
                return names;
            }
        };
        names.extend(resp.table_names().iter().cloned());
        match resp.last_evaluated_table_name() {
            Some(next) => start = Some(next.to_string()),
            None => break,
        }
    }
    names
}

/// Scan the lock table and return parallel (display, LockID) vectors for every item.
/// The Terraform lock item carries an `Info` attribute (who/operation/created).
async fn scan_locks(client: &Client, table: &str) -> (Vec<String>, Vec<String>) {
    let mut displays: Vec<String> = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    let mut start: Option<std::collections::HashMap<String, AttributeValue>> = None;

    loop {
        let resp = match client
            .scan()
            .table_name(table)
            .set_exclusive_start_key(start.clone())
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                println!("Error scanning table: {:?}", err);
                return (displays, ids);
            }
        };

        for item in resp.items() {
            let lock_id = match item.get("LockID").and_then(|v| v.as_s().ok()) {
                Some(id) => id.clone(),
                None => continue,
            };
            let info = item
                .get("Info")
                .and_then(|v| v.as_s().ok())
                .cloned()
                .unwrap_or_default();
            let display = if info.is_empty() {
                lock_id.clone()
            } else {
                format!("{}  |  {}", lock_id, info)
            };
            displays.push(display);
            ids.push(lock_id);
        }

        match resp.last_evaluated_key() {
            Some(key) => start = Some(key.clone()),
            None => break,
        }
    }

    (displays, ids)
}

pub async fn tf_unlock(ctx: &AwsCtx) {
    let config = ctx.config().await;
    let client = Client::new(&config);

    let tables = list_tables(&client).await;
    if tables.is_empty() {
        println!("No DynamoDB tables found");
        return;
    }
    let table = Listbox::new(&tables)
        .title("Which lock table?")
        .listbox_lines(10)
        .prompt()
        .unwrap()
        .run()
        .unwrap();

    let (displays, ids) = scan_locks(&client, &table).await;
    if ids.is_empty() {
        println!("No lock entries found in {}", table);
        return;
    }

    let choice = Listbox::new(&displays)
        .title("Which lock do you want to force-unlock?")
        .listbox_lines(10)
        .prompt()
        .unwrap()
        .run()
        .unwrap();
    let idx = displays.iter().position(|d| *d == choice).unwrap();
    let lock_id = ids[idx].clone();

    let mut confirm = Confirm::new(format!("Force-unlock (delete) lock '{}' ?", lock_id))
        .prompt()
        .unwrap();
    let confirm_string = match confirm.run() {
        Ok(value) => value,
        Err(_) => {
            println!("Aborted by user");
            return;
        }
    };
    drop(confirm);
    if confirm_string != "yes" && confirm_string != "y" {
        println!("Aborted by user");
        return;
    }

    match client
        .delete_item()
        .table_name(&table)
        .key("LockID", AttributeValue::S(lock_id.clone()))
        .send()
        .await
    {
        Ok(_) => println!("Lock '{}' removed", lock_id),
        Err(err) => println!("Failed to remove lock: {:?}", err),
    }
}
