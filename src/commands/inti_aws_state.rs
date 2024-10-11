use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_dynamodb::types::{AttributeDefinition, KeySchemaElement, KeyType, ScalarAttributeType};
use aws_sdk_s3::error::ProvideErrorMetadata;
use promkit::preset::readline::Readline;
use promkit::suggest::Suggest;

const AWS_REGION: [&str; 20] = [
    "us-east-1",
    "us-east-2",
    "us-west-1",
    "us-west-2",
    "ap-south-1",
    "ap-northeast-1",
    "ap-northeast-2",
    "ap-southeast-1",
    "ap-southeast-2",
    "ca-central-1",
    "eu-central-1",
    "eu-west-1",
    "eu-west-2",
    "eu-west-3",
    "eu-north-1",
    "sa-east-1",
    "cn-north-1",
    "cn-northwest-1",
    "us-gov-east-1",
    "us-gov-west-1",
];

async fn create_bucket(bucket_name: &str, region: String) -> bool {

    let region_provider = RegionProviderChain::first_try(Region::new(region.clone()));
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = aws_sdk_s3::Client::new(&config);

    let s3_cfg = aws_sdk_s3::types::CreateBucketConfiguration::builder()
        .location_constraint(aws_sdk_s3::types::BucketLocationConstraint::from(region.as_str()))
        .build();

    let response = client.create_bucket().create_bucket_configuration(s3_cfg).bucket(bucket_name).send().await;

    match response {
        Ok(_) => {
            println!("S3 bucket successfully created");
            true
        }
        Err(error) => {
            println!("Failed to create S3 bucket: \n {:?}", error);
            false
        }
    }
}

pub async fn create_table(
    table: &str,
    key: &str,
) -> bool {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);
    let a_name: String = key.into();
    let table_name: String = table.into();

    let ad = AttributeDefinition::builder()
        .attribute_name(&a_name)
        .attribute_type(ScalarAttributeType::S)
        .build().unwrap();

    let ks = KeySchemaElement::builder()
        .attribute_name(&a_name)
        .key_type(KeyType::Hash)
        .build().unwrap();



    let create_table_response = client
        .create_table()
        .table_name(table_name)
        .key_schema(ks)
        .billing_mode(aws_sdk_dynamodb::types::BillingMode::PayPerRequest)
        .attribute_definitions(ad)
        .send()
        .await;

    match create_table_response {
        Ok(_) => {
            println!("DynamoDB table successfully created");
            true
        }
        Err(error) => {
            println!("Failed to create dynamoDB table:\n {:?}", error.message().unwrap());
            false
        }
    }

}

pub async fn  init_aws_state() {

    let mut bucket_name = Readline::default()
        .title("How do you want to name the bucket?")
        .validator(
            |text| text.len() > 0,
            |text| format!("Your bucket name can't be empty {}", text.len()),
        )
        .prompt().unwrap();
    let bucket_name_string = bucket_name.run();
    let bucket_name_string = match bucket_name_string {
        Ok(value) => value,
        Err(_) => { print!("Aborted by user");std::process::exit(1); }
    };


    let mut dynamo = Readline::default()
        .title("How do you want to name the dynamoDB ?")
        .validator(
            |text| text.len() > 0,
            |text| format!("Your dynamoDB name can't be empty {}", text.len()),
        )
        .prompt().unwrap();
    let dynamo_string = dynamo.run();
    let dynamo_string = match dynamo_string {
        Ok(value) => value,
        Err(_) => { print!("Aborted by user");std::process::exit(1); }
    };

    let mut region = Readline::default()
        .title("Which region should I use ? (Press tab to see the list of available regions)")
        .enable_suggest(Suggest::from_iter(AWS_REGION))
        .validator(
            |text| AWS_REGION.contains(&text),
            |text| format!("You should enter a valid region {}", text),
        )
        .prompt().unwrap();
    let region_string = region.run();
    let region_string = match region_string {
        Ok(value) => value,
        Err(_) => { print!("Aborted by user");std::process::exit(1); }
    };

    drop(region);

    if create_bucket(bucket_name_string.as_str(), region_string).await == false {
        println!("\nFailed to create bucket, exiting");
        return;
    }

    if create_table(dynamo_string.as_str(), "LockID").await == false {
        println!("\nFailed to create dynamoDB table, exiting");
        return;
    }
}
