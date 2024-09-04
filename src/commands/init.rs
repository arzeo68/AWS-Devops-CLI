use std::fs;
use std::fs::File;
use std::io::Write;
use tera::{Context, Tera};

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

pub fn init(arg: &clap::ArgMatches) {
    println!("Initializing terraform repository...");

    let project = arg.get_one::<String>("project").unwrap();

    let environments = arg.get_many::<String>("environment")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();

    let accounts = arg.get_many::<String>("account")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();

    let mut path: String = arg.get_one::<String>("path").unwrap().to_string();
    if !path.ends_with("/") { path = format!("{}/", path); }

    let region: String = arg.get_one::<String>("region").unwrap_or(&"eu-west-1".to_string()).to_string();

    println!("Region: {:?}", region);

    println!("Path: {}", path);
    println!("Environments to create: {:?}", environments);
    println!("Accounts to create: {:?}", accounts);

    let folders_status = init_folders(path.as_str());
    match folders_status {
        Ok(_) => println!("Folders created successfully"),
        Err(e) => { println!("Error creating folders: {}", e);return },
    }

    for environment in environments {
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
        let tfvars_status = create_tfvars_file(format!("{}environments/{}/", path.as_str(), environment).as_str(), project, environment, region.as_str());
        if tfvars_status.is_err() {
            println!("Error creating tfvars file for environment: {}", tfvars_status.err().unwrap());return;
        }
        let main_status = create_main_file(format!("{}environments/{}/", path.as_str(), environment).as_str());
        if main_status.is_err() {
            println!("Error creating main file for environment: {}", main_status.err().unwrap());return;
        }
        println!("Environment {} created successfully", environment)
    }

    for account in accounts {
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
        let tfvars_status = create_tfvars_file(format!("{}accounts/{}/", path.as_str(), account).as_str(), project, account, region.as_str());
        if tfvars_status.is_err() {
            println!("Error creating tfvars file for environment: {}", tfvars_status.err().unwrap());return;
        }
        let main_status = create_main_file(format!("{}accounts/{}/", path.as_str(), account).as_str());
        if main_status.is_err() {
            println!("Error creating main file for environment: {}", main_status.err().unwrap());return;
        }
        println!("Account {} created successfully", account)
    }
}
