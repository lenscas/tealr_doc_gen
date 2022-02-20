pub(crate) struct Paths {
    pub(crate) json: String,
    pub(crate) name: String,
    pub(crate) root: String,
    pub(crate) build_dir: String,
}

pub(crate) fn get_paths() -> Paths {
    let matches = clap::App::new("tealr doc gen")
        .arg(
            clap::Arg::new("json")
                .long("json")
                .short('j')
                .takes_value(true)
                .help("Path to the json file")
                .required(true),
        )
        .arg(
            clap::Arg::new("name")
                .long("name")
                .short('n')
                .takes_value(true)
                .help("Name of the library")
                .required(true),
        )
        .arg(
            clap::Arg::new("root")
                .long("root")
                .short('r')
                .takes_value(true)
                .help("The root that the pages link to.")
                .default_value("./")
                .default_missing_value("./"),
        )
        .arg(
            clap::Arg::new("build_folder")
                .long("build_folder")
                .short('b')
                .takes_value(true)
                .help("In which folder to store the generated pages")
                .default_value("./pages")
                .default_missing_value("./pages"),
        )
        .get_matches();

    let json = matches.value_of("json").unwrap().to_owned();
    let name = matches.value_of("name").unwrap().to_owned();
    let root = matches.value_of("root").unwrap().to_owned();
    let build_dir = matches.value_of("build_folder").unwrap().to_owned();
    Paths {
        name,
        root,
        json,
        build_dir,
    }
}
