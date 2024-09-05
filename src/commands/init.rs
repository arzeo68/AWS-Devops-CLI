use std::fs;
use std::fs::File;
use std::io::Write;
use colored::Colorize;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use tera::{Context, Tera};
use comfy_table::{ContentArrangement, Table};
use promkit::preset::confirm::Confirm;
use promkit::preset::readline::Readline;
use promkit::suggest::Suggest;

struct InitArgs {
    project: String,
    environments: Vec<String>,
    accounts: Vec<String>,
    region: String,
    path: String,
    status: bool,
}


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

fn create_main_file(path: &str) -> std::io::Result<()> {
    let main_content: &str = "\
provider \"aws\" {
  region = var.region
  default_tags {
    tags = {
      \"environment-tier\" = var.environment
      \"project\"          = var.project
      \"${var.project}-backup\"      = \"None\"
    }
  }
}

locals {
    project = var.project
    environment = var.environment
    name_prefix = \"${var.project}-${var.environment}\"
}
";
    let mut file = File::create(format!("{}main.tf", path))?;
    file.write_all(main_content.as_bytes())?;
    Ok(())
}

fn create_tfvars_file(path: &str, project_name: &str, environment_name: &str, region: &str) -> std::io::Result<()> {

    let tfvars_content: String = format!("project = \"{}\"\nenvironment = \"{}\"\nregion = {:?}", project_name, environment_name, region);

    let mut file = File::create(format!("{}terraform.tfvars", path))?;
    file.write_all(tfvars_content.as_bytes())?;
    Ok(())
}

fn create_outputs_file(path: &str) -> std::io::Result<()> {
    let mut file = File::create(format!("{}outputs.tf", path))?;
    file.write_all(b"")?;
    Ok(())
}

fn create_inputs_file(path: &str) -> std::io::Result<()> {

    let inputs_content: &str = "\
variable \"project\" {
    description = \"Project name\"
}\
\n
variable \"environment\" {
    description = \"Environment name\"
}\
\n\
variable \"region\" {
    description = \"AWS region\"
    default = \"eu-west-1\"
}\
";

        let mut file = File::create(format!("{}inputs.tf", path))?;
        file.write_all(inputs_content.as_bytes())?;
        Ok(())
}

fn create_backend_file(path: &str, region: &str) -> std::io::Result<()> {

    let mut tera = Tera::default();
    let mut context = Context::new();
    context.insert("region", region);
    let backend_content = tera.render_str("\
terraform {
    backend \"s3\" {
        bucket = \"mybucket\"
        key    = \"path/to/my/key\"
        region = \"{{ region }}\"
        dynamodb_table = \"tfstate-lock\"
        encrypt        = true
    }
}", &context).unwrap();


    let mut file = File::create(format!("{}backend.tf", path))?;
    file.write_all(backend_content.as_bytes())?;

    Ok(())
}

fn init_environments(root: &str, environment: &str) -> std::io::Result<()> {
    fs::create_dir_all(format!("{}environments/{}", root, environment))?;
    Ok(())
}

fn init_accounts(root: &str, account: &str) -> std::io::Result<()> {
    fs::create_dir_all(format!("{}accounts/{}", root, account))?;
    Ok(())
}
fn init_folders(root: &str) -> std::io::Result<()> {

    fs::create_dir_all(root)?;
    fs::create_dir_all(format!("{}environments", root))?;
    fs::create_dir_all(format!("{}accounts", root))?;
    fs::create_dir_all(format!("{}modules", root))?;

    Ok(())
}

fn display_prompt() -> InitArgs {
    let mut project = Readline::default()
        .title("What is the name of the project ?")
        .validator(
            |text| text.len() > 0,
            |text| format!("You should put a name {}", text.len()),
        )
        .prompt().unwrap();
    let project_string = project.run().unwrap();

    let mut environments = Readline::default()
        .title("Which environment do you want ? (If you want multiple environments, separate them by a comma)")
        .validator(
            |text| text.len() > 0,
            |text| format!("You should at least enter one environment name {}", text.len()),
        )
        .prompt().unwrap();
    let environments_string = environments.run().unwrap();


    let mut accounts = Readline::default()
        .title("Which account do you want ? (If you want multiple accounts, separate them by a comma)")
        .validator(
            |text| text.len() > 0,
            |text| format!("You should at least enter one account name {}", text.len()),
        )
        .prompt().unwrap();
    let accounts_string = accounts.run().unwrap();

    let mut region = Readline::default()
        .title("Which region should I use ? (Press tab to see the list of available regions)")
        .enable_suggest(Suggest::from_iter(AWS_REGION))
        .validator(
            |text| AWS_REGION.contains(&text),
            |text| format!("You should enter a valid region {}", text),
        )
        .prompt().unwrap();
    let region_string = region.run().unwrap();

    let mut path = Readline::default()
        .title("Where should I create the repository ? (Default: current directory)")
        .prompt().unwrap();
    let mut path_string = path.run().unwrap();
    if path_string.is_empty() {
        path_string = "./".to_string();
    }
    drop(path);

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .add_row(vec![
            "Region",
            region_string.as_str()
        ])
        .add_row(vec![
            "Project",
            project_string.as_str()
        ])
        .add_row(vec![
            "Path",
            path_string.as_str()
        ])
        .add_row(vec![
            "Environments",
            environments_string.as_str()
        ])
        .add_row(vec![
            "Accounts",
            accounts_string.as_str()
        ]);

    println!("{}", table);

    let mut confirm = Confirm::new("Is this correct ?").prompt().unwrap();
    let confirm_string = confirm.run().unwrap();
    let status = confirm_string == "yes" || confirm_string == "y";
    drop(confirm);

    InitArgs {
        project: project_string,
        environments: environments_string.split(",").map(|v| v.trim().to_string()).collect(),
        accounts: accounts_string.split(",").map(|v| v.trim().to_string()).collect(),
        region: region_string,
        path: path_string,
        status,
    }
}

pub fn init() {

    let args = display_prompt();
    if !args.status {
        println!("Aborted by user");
        return;
    }
    let project = args.project;
    let environments = args.environments;
    let accounts = args.accounts;
    let region = args.region;
    let mut path = args.path;
    if !path.ends_with("/") { path = format!("{}/", path); }

    let folders_status = init_folders(path.as_str());
    if folders_status.is_err() {
        println!("Error creating folders: {}", folders_status.err().unwrap());return;
    }


    for environment in &environments {
        let environments_status = init_environments(path.as_str(), environment);
        if environments_status.is_err() {
            println!("Error creating environments: {}", environments_status.err().unwrap());return;
        }
        let backend_status = create_backend_file(format!("{}environments/{}/", path.as_str(), environment).as_str(), region.as_str());
        if backend_status.is_err() {
            println!("Error creating backend file for environment: {}", backend_status.err().unwrap());return;
        }
        let inputs_status = create_inputs_file(format!("{}environments/{}/", path.as_str(), environment).as_str());
        if inputs_status.is_err() {
            println!("Error creating inputs file for environment: {}", inputs_status.err().unwrap());return;
        }
        let outputs_status = create_outputs_file(format!("{}environments/{}/", path.as_str(), environment).as_str());
        if outputs_status.is_err() {
            println!("Error creating outputs file for environment: {}", outputs_status.err().unwrap());return;
        }
        let tfvars_status = create_tfvars_file(format!("{}environments/{}/", path.as_str(), environment).as_str(), project.as_str(), environment, region.as_str());
        if tfvars_status.is_err() {
            println!("Error creating tfvars file for environment: {}", tfvars_status.err().unwrap());return;
        }
        let main_status = create_main_file(format!("{}environments/{}/", path.as_str(), environment).as_str());
        if main_status.is_err() {
            println!("Error creating main file for environment: {}", main_status.err().unwrap());return;
        }
    }

    for account in &accounts {
        let accounts_status = init_accounts(path.as_str(), account);
        if accounts_status.is_err() {
            println!("Error creating accounts: {}", accounts_status.err().unwrap());return;
        }
        let backend_status = create_backend_file(format!("{}accounts/{}/", path.as_str(), account).as_str(), region.as_str());
        if backend_status.is_err() {
            println!("Error creating backend file for environment: {}", backend_status.err().unwrap());return;
        }
        let inputs_status = create_inputs_file(format!("{}accounts/{}/", path.as_str(), account).as_str());
        if inputs_status.is_err() {
            println!("Error creating inputs file for environment: {}", inputs_status.err().unwrap());return;
        }
        let outputs_status = create_outputs_file(format!("{}accounts/{}/", path.as_str(), account).as_str());
        if outputs_status.is_err() {
            println!("Error creating outputs file for environment: {}", outputs_status.err().unwrap());return;
        }
        let tfvars_status = create_tfvars_file(format!("{}accounts/{}/", path.as_str(), account).as_str(), project.as_str(), account, region.as_str());
        if tfvars_status.is_err() {
            println!("Error creating tfvars file for environment: {}", tfvars_status.err().unwrap());return;
        }
        let main_status = create_main_file(format!("{}accounts/{}/", path.as_str(), account).as_str());
        if main_status.is_err() {
            println!("Error creating main file for environment: {}", main_status.err().unwrap());return;
        }
    }
    println!("Init command executed {}!", "successfully".green().bold());
}
