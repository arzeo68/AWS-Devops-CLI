use std::fs;
use std::fs::File;
use std::io::Write;
use colored::Colorize;

fn create_main_file(path: &str) -> std::io::Result<()> {
    let mut file = File::create(format!("{}main.tf", path))?;
    file.write_all(b"")?;
    Ok(())
}

fn create_outputs_file(path: &str) -> std::io::Result<()> {
    let mut file = File::create(format!("{}outputs.tf", path))?;
    file.write_all(b"")?;
    Ok(())
}

fn create_inputs_file(path: &str) -> std::io::Result<()> {

    let inputs_content: &str = "\
variable \"name_prefix\" {
    description = \"Prefix to apply on every resources\"
}\
";

    let mut file = File::create(format!("{}inputs.tf", path))?;
    file.write_all(inputs_content.as_bytes())?;
    Ok(())
}

fn create_module_folder(root: &str, module_name: &str) -> std::io::Result<()> {

    fs::create_dir_all(format!("{}{}", root, module_name))?;
    Ok(())
}

pub fn module(arg: &clap::ArgMatches) {
    let module_name: String = arg.get_one::<String>("name").unwrap().to_string();


    let mut path: String = arg.get_one::<String>("path").unwrap().to_string();
    if !path.ends_with("/") { path = format!("{}/", path); }

    let folders_status = create_module_folder(path.as_str(), module_name.as_str());
    if folders_status.is_err() {
        println!("Error creating module folder: {}", folders_status.err().unwrap());
    }
    let inputs_status = create_inputs_file(format!("{}{}/", path, module_name).as_str());
    if inputs_status.is_err() {
        println!("Error creating inputs file: {}", inputs_status.err().unwrap());
    }
    let outputs_status = create_outputs_file(format!("{}{}/", path, module_name).as_str());
    if outputs_status.is_err() {
        println!("Error creating outputs file: {}", outputs_status.err().unwrap());
    }
    let main_status = create_main_file(format!("{}{}/", path, module_name).as_str());
    if main_status.is_err() {
        println!("Error creating main file: {}", main_status.err().unwrap());
    }
    println!("Module {} created {}", module_name.bold(), "successfully".green().bold());
}
