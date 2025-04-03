use std::{
    fs::{create_dir_all, read_to_string},
    path::Path,
};

use anyhow::Context;
use tealr::{RecordGenerator, TypeGenerator, TypeWalker};

use crate::{
    app::{DefTemplateConfig, DefTemplateKind, LuaAddon},
    Paths,
};

use super::{
    definition_file::create_d_file,
    generate_warnings::warn_about_missing_exports,
    html::{
        create_globals_docs, run_and_write, CustomPage, GlobalInstancesDoc, IndexPage, TypeDesc,
        TypeOrPage,
    },
    lua_addon::create_lua_addon,
    sidebar::generate_sidebar_data,
};

pub(crate) fn run_from_walker(
    mut paths: Paths,
    type_defs: TypeWalker,
) -> Result<(), anyhow::Error> {
    warn_about_missing_exports(&type_defs);
    let write_path = Path::new(&paths.build_dir).join(&paths.root);
    create_dir_all(&write_path)?;
    let definition_file_storage =
        create_d_file(type_defs.clone(), write_path.clone(), &paths.name, &paths)?;
    let lua_addon_storage = create_lua_addon(
        paths.lua_addon.clone().unwrap_or(LuaAddon::False),
        type_defs.clone(),
        definition_file_storage.clone(),
        paths.is_global,
        &paths.name,
    )
    .context("Failed generating lua language server addon")?;

    if lua_addon_storage {
        paths.def_config.templates.insert(
            "lua language server".into(),
            DefTemplateConfig {
                extension: ".zip".into(),
                template: DefTemplateKind::Custom("Lua language server".into()),
            },
        );
    }

    let link_path = Path::new("/").join(&paths.root);

    let mut z = RecordGenerator::new::<RecordGenerator>(true);
    let sidebar = generate_sidebar_data(&type_defs, &paths, &link_path);
    for type_def in type_defs.iter() {
        let users = crate::find_uses::find_users(type_def, &type_defs);
        match type_def {
            TypeGenerator::Record(x) => {
                let x = x.to_owned();
                if x.should_be_inlined {
                    z.documentation.extend(x.documentation);
                    z.fields.extend(x.fields);
                    z.functions.extend(x.functions);
                    z.meta_function.extend(x.meta_function);
                    z.meta_function_mut.extend(x.meta_function_mut);
                    z.meta_method.extend(x.meta_method);
                    z.meta_method_mut.extend(x.meta_method_mut);
                    z.methods.extend(x.methods);
                    z.mut_functions.extend(x.mut_functions);
                    z.mut_methods.extend(x.mut_methods);
                    z.static_fields.extend(x.static_fields);
                    z.type_doc += &x.type_doc;
                    continue;
                }
            }
            TypeGenerator::Enum(_) => (),
        }
        let definition_templates = paths.def_config.templates.clone();
        let (template_runner, docs_instance) =
            create_globals_docs(&paths.template_kind, type_def, |config| {
                GlobalInstancesDoc {
                    side_bar: sidebar.clone(),
                    link_path: link_path.clone(),
                    etlua: config.etlua,
                    template: config.template,
                    page: TypeOrPage::Type(TypeDesc {
                        type_members: type_def.to_owned(),
                        type_name: config.type_name.to_string(),
                        used_by: users,
                    }),
                    all_types: Some(type_defs.given_types.clone()),
                    globals: Some(type_defs.global_instances_off.clone()),
                    def_files: definition_templates.clone(),
                    library_name: paths.name.clone(),
                    definition_files_folder: definition_file_storage.to_string_lossy().to_string(),
                }
            })?;
        let name = match &docs_instance.page {
            TypeOrPage::Type(x) => x.type_name.clone(),
            _ => unreachable!(),
        };
        run_and_write(&write_path, &template_runner, docs_instance)
            .with_context(|| format!("Failed while generating file for: {name}"))?;
    }
    let type_def = TypeGenerator::Record(Box::new(z));
    let (template_runner, mut docs_instance) =
        create_globals_docs(&paths.template_kind, &type_def, |config| {
            GlobalInstancesDoc {
                side_bar: sidebar.clone(),
                link_path,
                etlua: config.etlua,
                template: config.template,
                page: TypeOrPage::IndexPage(IndexPage {
                    type_members: type_def.to_owned(),
                    type_name: config.type_name.to_string(),
                    all_types: type_defs.given_types.clone(),
                }),
                all_types: Some(type_defs.given_types.clone()),
                globals: Some(type_defs.global_instances_off.clone()),
                def_files: paths.def_config.templates.clone(),
                library_name: paths.name.clone(),
                definition_files_folder: definition_file_storage.to_string_lossy().to_string(),
            }
        })?;
    run_and_write(&write_path, &template_runner, docs_instance.clone())
        .context("Error while generating file for the index page")?;
    for custom in &type_defs.extra_page {
        docs_instance.page = TypeOrPage::CustomPage(CustomPage {
            name: custom.name.clone(),
            markdown_content: custom.content.clone(),
        });
        run_and_write(&write_path, &template_runner, docs_instance.clone()).with_context(|| {
            format!("Error while generating custom page named: {}", custom.name)
        })?;
    }
    Ok(())
}

pub(crate) fn run_template(paths: Paths) -> Result<(), anyhow::Error> {
    let json = read_to_string(&paths.json)
        .with_context(|| format!("Failed reading type json at location: {}", paths.json))?;
    let value = serde_json::from_str::<serde_json::Value>(&json)
        .with_context(|| format!("Failed deserializing given type file at: {}", paths.json))?;
    let type_defs: tealr::TypeWalker = match serde_json::from_value::<TypeWalker>(value.clone()) {
        Ok(x) => {
            if !x.check_correct_version() {
                eprintln!("Warning:");
                eprintln!("Tealr version used to create this json is not equal to the tealr version used to build this version of tealr_doc_gen");
                eprintln!("Tealr version used: {}", x.get_tealr_version_used());
                eprintln!("built version used: {}", tealr::get_tealr_version());
                eprintln!("Please update both tealr and tealr_doc_gen so the versions match.");
                eprintln!("Schema seems compatible. Trying anyway");
            }
            x
        }
        Err(x) => match x.classify() {
            serde_json::error::Category::Data => {
                match value.get::<String>("tealr_version_used".into()) {
                    Some(serde_json::Value::String(y)) => {
                        if y != tealr::get_tealr_version() {
                            return Err(x).context(format!("Tealr version used to create the json is not compatible with this version of tealr_doc_gen.\nTealr version used: {y}\nCompatible tealr_version: {}.\nSchema error:", tealr::get_tealr_version()));
                        } else {
                            return Err(x.into());
                        }
                    }
                    Some(_) | None => {
                        return Err(x).context("Json is not the correct schema.\nCould not get the tealr version used to create the json.\nError in schema:");
                    }
                }
            }
            _ => return Err(x.into()),
        },
    };

    run_from_walker(paths, type_defs)
}
