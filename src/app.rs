use std::{collections::HashMap, fs::read_to_string};

use anyhow::Context;
use clap::{Arg, Command};
use tealr::ToTypename;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
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

#[derive(Clone)]
pub(crate) struct Paths {
    pub(crate) json: String,
    pub(crate) name: String,
    pub(crate) root: String,
    pub(crate) build_dir: String,
    pub(crate) template_kind: TemplateKind,
    pub(crate) def_config: TypeDefFile,
    pub(crate) is_global: bool,
    pub(crate) lua_addon: Option<LuaAddon>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum DefTemplateRunnerKind {
    Builtin,
    Custom(String),
}
#[derive(
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    tealr::mlu::FromToLua,
    ToTypename,
)]
/// Template used to generate a definition file
pub enum DefTemplateKind {
    Teal,
    Custom(String),
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    tealr::mlu::FromToLua,
    ToTypename,
)]
/// The configuration for the definition files that get generated.
///
/// Contain the information needed to link to them
pub struct DefTemplateConfig {
    /// File extension used
    pub(crate) extension: String,
    /// template used
    pub(crate) template: DefTemplateKind,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct TypeDefFile {
    pub(crate) runner: DefTemplateRunnerKind,
    pub(crate) templates: HashMap<String, DefTemplateConfig>,
}

impl Default for TypeDefFile {
    fn default() -> Self {
        Self {
            runner: DefTemplateRunnerKind::Builtin,
            templates: {
                let mut files = HashMap::new();
                files.insert(
                    "teal".into(),
                    DefTemplateConfig {
                        extension: ".d.tl".into(),
                        template: DefTemplateKind::Teal,
                    },
                );
                files
            },
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum LuaAddon {
    False,
    Create {
        words: Vec<String>,
        files: Vec<String>,
        settings: HashMap<String, serde_json::Value>,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    doc_template: TemplateKind,
    page_root: String,
    store_in: String,
    name: String,
    type_def_files: TypeDefFile,
    lua_addon: Option<LuaAddon>,
    is_global: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            doc_template: Default::default(),
            page_root: Default::default(),
            is_global: true,
            store_in: "pages".into(),
            name: "Your_API".into(),
            type_def_files: Default::default(),
            lua_addon: Some(LuaAddon::Create {
                words: Vec::new(),
                files: Vec::new(),
                settings: HashMap::new(),
            }),
        }
    }
}

pub(crate) enum Modes {
    Credits,
    GenerateDocs(Box<Paths>),
    SelfDocTemplate {
        build_dir: String,
    },
    SelfDefTemplate {
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
                Command::new("gen-self")
                    .about("Generates files used to add custom behavior to tealr_doc_gen")
                    .arg(Arg::new("docs_documentation_template").long("docs-documentation-template").help(
                        "Generates the documentation for the lua api exposed to the template used to generate documentation pages.",
                    )).arg(
                        Arg::new("docs_definition_template")
                        .long("docs-definition-template")
                        .help("Generates the documentation for the lua api exposed to the template used to generate definition files.")
                    )
                    .arg(
                        Arg::new("config")
                            .long("config")
                            .help("Generates a new config file"),
                    )
                    .arg(Arg::new("doc_template").long("doc-template").help(
                        "Generates the default template used to generate the documentation pages",
                    ))
                    .arg(
                        Arg::new("lua_runner")
                            .long("lua-runner")
                            .help("Generate the lua code that loads and executes the template."),
                    )
                    .arg(
                        Arg::new("definition_template")
                            .long("definition-template")
                            .help("Generates the default template used to generate the teal definition file.")
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
        return Ok(Modes::GenerateDocs(Box::new(Paths {
            is_global: config.is_global,
            json: config.name.clone() + ".json",
            build_dir: config.store_in,
            name: config.name,
            root: config.page_root,
            template_kind: config.doc_template,
            def_config: config.type_def_files,
            lua_addon: config.lua_addon,
        })));
    }
    if let Some(x) = matches.subcommand_matches("gen-self") {
        if x.contains_id("docs_documentation_template") {
            let config = read_config()?;
            return Ok(Modes::SelfDocTemplate {
                build_dir: config.store_in,
            });
        } else if x.contains_id("docs_definition_template") {
            let config = read_config()?;
            return Ok(Modes::SelfDefTemplate {
                build_dir: config.store_in,
            });
        }
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
        } else if x.contains_id("definition_template") {
            (
                include_str!("../base_teal_definition_template.etlua").into(),
                "./teal_definition_template.etlua",
            )
        } else if x.contains_id("lua_runner") {
            (
                include_str!("../base_run_template.lua").into(),
                "./run_template.lua",
            )
        } else {
            return Err(anyhow::anyhow!("Missing argument"));
        };
        if x.contains_id("print") {
            println!("{}", text);
            return Ok(Modes::Nothing);
        }
        return Ok(Modes::GenFile {
            file: text,
            location,
        });
    }
    Ok(Modes::Nothing)
}

fn read_config() -> Result<Config, anyhow::Error> {
    serde_json::from_str(
        &read_to_string("./tealr_doc_gen_config.json")
            .context("Could not read tealr_doc_gen_config.json in current directory. Maybe generate one using `tealr_doc_gen gen_self --config`?")?
    ).context("Error while parsing the config file. Use `tealr_doc_gen gen_self --config` to generate an example")
}
