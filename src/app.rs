use std::fs::read_to_string;

use anyhow::Context;
use clap::{Arg, Command};

#[derive(serde::Serialize, serde::Deserialize)]
pub enum TemplateKind {
    Builtin,
    FromLua(String),
    FromTemplate(String),
}

impl Default for TemplateKind {
    fn default() -> Self {
        Self::Builtin
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    template: TemplateKind,
    page_root: String,
    store_in: String,
    name: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            template: Default::default(),
            page_root: Default::default(),
            store_in: "pages".into(),
            name: "Your_API".into(),
        }
    }
}

pub(crate) struct Paths {
    pub(crate) json: String,
    pub(crate) name: String,
    pub(crate) root: String,
    pub(crate) build_dir: String,
    pub(crate) template_kind: TemplateKind,
}

pub(crate) enum Modes {
    Credits,
    GenerateDocs(Paths),
    SelfDocs {
        build_dir: String,
    },
    GenFile {
        file: String,
        location: &'static str,
    },
    Nothing,
}

pub(crate) fn get_paths() -> Result<Modes, anyhow::Error> {
    let matches =
        clap::App::new("tealr doc gen")
            .subcommand(
                Command::new("run")
                    .alias("gen")
                    .about("Generates the documentation pages"),
            )
            .subcommand(
                Command::new("gen_self")
                    .about("Generates files used to add custom behavior to tealr_doc_gen")
                    .arg(Arg::new("lua_docs").long("lua_docs").help(
                        "Generates the documentation for the lua api exposed by tealr_doc_gen.",
                    ))
                    .arg(
                        Arg::new("config")
                            .long("config")
                            .help("Generates a new config file"),
                    )
                    .arg(Arg::new("doc_template").long("doc_template").help(
                        "Generates the default template used to generate the documentation pages",
                    ))
                    .arg(
                        Arg::new("doc_lua_kickstart")
                            .long("doc_lua_kickstart")
                            .help("Generates the lua code used to build the template"),
                    )
                    .arg(
                        Arg::new("print")
                            .long("print")
                            .short('p')
                            .help("Prints the file instead of writing it directly to a file"),
                    ),
            )
            .arg(
                Arg::new("credits")
                    .long("credits")
                    .short('c')
                    .help("Shows who worked on tealr_doc_gen"),
            )
            .get_matches();
    if matches.contains_id("credits") {
        return Ok(Modes::Credits);
    }

    if matches.subcommand_matches("run").is_some() {
        let config: Config = read_config()?;
        return Ok(Modes::GenerateDocs(Paths {
            json: config.name.clone() + ".json",
            build_dir: config.store_in,
            name: config.name,
            root: config.page_root,
            template_kind: config.template,
        }));
    }
    if let Some(x) = matches.subcommand_matches("gen_self") {
        return Ok(if x.contains_id("lua_docs") {
            let config = read_config()?;
            Modes::SelfDocs {
                build_dir: config.store_in,
            }
        } else {
            let (text, location) = if x.contains_id("config") {
                (
                    serde_json::to_string_pretty(&Config::default())?,
                    "./tealr_doc_gen_config.json",
                )
            } else if x.contains_id("doc_template") {
                (
                    include_str!("../base_template.etlua").into(),
                    "./template.etlua",
                )
            } else if x.contains_id("doc_lua_kickstart") {
                (
                    include_str!("../base_run_template.lua").into(),
                    "./run_template.lua",
                )
            } else {
                return Err(anyhow::anyhow!("Missing argument"));
            };
            if x.contains_id("print") {
                println!("{}", text);
                Modes::Nothing
            } else {
                Modes::GenFile {
                    file: text,
                    location,
                }
            }
        });
    };
    Ok(Modes::Nothing)
}

fn read_config() -> Result<Config, anyhow::Error> {
    serde_json::from_str(
        &read_to_string("./tealr_doc_gen_config.json")
            .context("Could not read tealr_doc_gen_config.json in current directory. Maybe generate one using `tealr_doc_gen gen_self --config`?")?
    ).context("Error while parsing the config file. Use `tealr_doc_gen gen_self --config` to generate an example")
}
