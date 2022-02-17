pub(crate) struct Paths {
    pub(crate) json: String,
    pub(crate) name: String,
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
        .get_matches();

    let json = matches.value_of("json").unwrap().to_owned();
    let name = matches.value_of("name").unwrap().to_owned();
    Paths { name, json }
}
