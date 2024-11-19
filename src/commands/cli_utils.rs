use promkit::preset::listbox::Listbox;

pub(crate) fn select_type() -> String {
    let types = vec!["EC2", "ECS container"];
    Listbox::new(&types)
        .title("Select the type of resource you want to port forward")
        .listbox_lines(5)
        .prompt().unwrap().run().unwrap()
}

pub(crate) fn get_index_of<T: PartialEq>(vec: &Vec<T>, value: T) -> usize {
    vec.iter().position(|x| *x == value).unwrap()
}