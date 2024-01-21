use std::{
    borrow::Cow,
    collections::HashMap,
    fs::{create_dir_all, read_to_string},
    path::{Path, PathBuf},
};

use anyhow::Context;
use tealr::{
    mlu::{self, ExportInstances, FromToLua, TealData, TypedFunction},
    type_parts_to_str, GlobalInstance, NameContainer, RecordGenerator, ToTypename, TypeGenerator,
    TypeWalker,
};

use crate::{
    app::{DefTemplateConfig, DefTemplateKind, LuaAddon, Paths, TemplateKind},
    create_lua_addon::create_lua_addon,
    doc_gen::{get_type_name, type_should_be_inlined},
    markdown::MarkdownEvent,
};

#[derive(FromToLua, ToTypename, Clone)]
/// An element in the sidebar
struct SideBar {
    /// What url to link to
    link_to: String,
    /// Name of the type in the sidebar
    name: String,
    /// The members of the type
    members: Vec<Members>,
}
#[derive(FromToLua, ToTypename, Clone)]
/// A member as shown in the sidebar
struct Members {
    /// The name of the member
    name: NameContainer,
}

#[derive(tealr::mlu::TealDerive)]
struct TestDocs;
impl TealData for TestDocs {
    fn add_methods<'lua, T: mlu::TealDataMethods<'lua, Self>>(methods: &mut T) {
        methods.document("```rs");
        methods.document("This should not be visible");
        methods.document("```");
        methods.document("```teal_lua");
        methods.document("local a:number = 1");
        methods.document("```");
        methods.document("some documentation for the next method");
        methods.add_method("test", |_, _, ()| Ok(1));

        methods.document_type("some documentation for the entire type");
    }

    fn add_fields<'lua, F: mlu::TealDataFields<'lua, Self>>(fields: &mut F) {
        fields.document("some docs for the next field");
        fields.add_field_function_get("a", |_, _| Ok(1))
    }
}

#[derive(Clone, FromToLua, ToTypename)]
struct TypeDesc {
    type_members: TypeGenerator,
    type_name: String,
    used_by: Vec<crate::find_uses::User>,
}
#[derive(Clone, FromToLua, ToTypename)]
struct IndexPage {
    all_types: Vec<TypeGenerator>,
    type_name: String,
    type_members: TypeGenerator,
}
#[derive(Clone, FromToLua, ToTypename)]
struct CustomPage {
    name: String,
    markdown_content: String,
}
#[derive(Clone, FromToLua, ToTypename)]
enum TypeOrPage {
    Type(TypeDesc),
    IndexPage(IndexPage),
    CustomPage(CustomPage),
}

#[derive(Clone)]
struct GlobalInstancesDoc {
    side_bar: Vec<SideBar>,
    link_path: PathBuf,
    etlua: String,
    template: String,
    page: TypeOrPage,
    all_types: Option<Vec<TypeGenerator>>,
    globals: Option<Vec<GlobalInstance>>,
    def_files: HashMap<String, DefTemplateConfig>,
    library_name: String,
    definition_files_folder: String,
}
impl Default for GlobalInstancesDoc {
    fn default() -> Self {
        Self {
            side_bar: Default::default(),
            link_path: Default::default(),
            etlua: Default::default(),
            template: Default::default(),
            page: TypeOrPage::CustomPage(CustomPage {
                name: Default::default(),
                markdown_content: Default::default(),
            }),
            all_types: Default::default(),
            globals: Default::default(),
            def_files: Default::default(),
            library_name: Default::default(),
            definition_files_folder: Default::default(),
        }
    }
}

type MarkdownEventTable = Vec<MarkdownEvent>;
type OptionalMarkdownEvent = Option<MarkdownEvent>;

tealr::create_union_mlua!(pub enum MarkdownTransformation = MarkdownEventTable | OptionalMarkdownEvent );

impl ExportInstances for GlobalInstancesDoc {
    fn add_instances<'lua, T: mlu::InstanceCollector<'lua>>(
        self,
        instance_collector: &mut T,
    ) -> mlu::mlua::Result<()> {
        let side_bar = self.side_bar;
        let link_path = self.link_path;
        let etlua = self.etlua;
        let template = self.template;
        let all_types = self.all_types;
        let globals = self.globals;
        let definition_files_folder = self.definition_files_folder;

        instance_collector.add_instance("side_bar_types", move |_| Ok(side_bar))?;
        instance_collector.add_instance("etlua", move |lua| {
            lua.load(&etlua).set_name("etlua").into_function()
        })?;
        instance_collector.add_instance("template", move |_| Ok(template))?;
        instance_collector.add_instance("page", move |_| Ok(self.page))?;
        instance_collector.add_instance("globals", move |_| Ok(globals))?;
        let all_types_2 = all_types.clone();
        instance_collector.add_instance("all_types", move |_| Ok(all_types_2))?;
        instance_collector.add_instance("create_link", move |lua| {
            let link = link_path;
            TypedFunction::from_rust(
                move |_, name: String| {
                    let (name, hash) = name
                        .find('#')
                        .map(|x| name.split_at(x))
                        .unwrap_or((&name, ""));
                    let is_index = match &all_types {
                        None => false,
                        Some(x) => x.iter().any(|x| match x {
                            TypeGenerator::Record(x) => {
                                x.should_be_inlined
                                    && type_parts_to_str(x.type_name.clone()) == name
                            }
                            TypeGenerator::Enum(_) => false,
                        }),
                    };
                    let name = if is_index { "index" } else { name };

                    Ok(link
                        .join(name.to_string() + ".html" + hash)
                        .to_string_lossy()
                        .to_string())
                },
                lua,
            )
        })?;
        instance_collector
            .add_instance("definition_file_folder", |_| Ok(definition_files_folder))?;
        instance_collector.add_instance("markdown_codeblock_kind_creator", |_| {
            Ok(crate::markdown::MarkdownCodeBlockKindCreator {})
        })?;
        instance_collector.add_instance("markdown_event_creator", |_| {
            Ok(crate::markdown::MarkdownEventCreator {})
        })?;
        instance_collector.add_instance("markdown_tag_creator", |_| {
            Ok(crate::markdown::MarkdownTagCreator {})
        })?;

        instance_collector.add_instance("library_name", |_| Ok(self.library_name))?;
        instance_collector.add_instance("definition_config", |_| Ok(self.def_files))?;
        instance_collector.document_instance(
            "defines what types make use of the current type and how they use it",
        );

        instance_collector.document_instance("Removes all duplicate instances of a table using a function to select what to look for. Returns a new table with the duplicates removed");
        instance_collector.document_instance("```teal_lua");
        instance_collector.document_instance("local with_dupes = {1,2,3,3,2,1}");
        instance_collector.document_instance(
            "local without_duplicates = dedupe_by(with_dupes,function(x:integer):integer return x end)",
        );
        instance_collector.document_instance("print(#without_duplicates, #with_dupes)");
        instance_collector.document_instance("```");
        instance_collector.add_instance("dedupe_by", |lua| {
            TypedFunction::from_rust(
                |lua,
                 (a, b): (
                    Vec<mlu::generics::X>,
                    TypedFunction<mlu::generics::X, mlu::generics::Y>,
                )| {
                    let table = lua.create_table()?;
                    a.into_iter()
                        .filter_map(|x| match b.call(x.clone()) {
                            Ok(to_store) => match table.contains_key(to_store.clone()) {
                                Ok(true) => None,
                                Ok(false) => match table.set(to_store, true) {
                                    Ok(()) => Some(Ok(x)),
                                    Err(x) => Some(Err(x)),
                                },
                                Err(x) => Some(Err(x)),
                            },
                            Err(x) => Some(Err(x)),
                        })
                        .collect::<Result<Vec<_>, _>>()
                },
                lua,
            )
        })?;
        instance_collector.add_instance("parse_markdown", |lua| {
            TypedFunction::from_rust(
                |_,
                 (markdown, func): (
                    String,
                    Option<TypedFunction<MarkdownEvent, MarkdownTransformation>>,
                )| match func {
                    Some(x) => crate::markdown::parse_markdown_lua(
                        markdown,
                        |event| -> Result<Vec<MarkdownEvent>, mlu::mlua::Error> {
                            let z = x.call(event)?;
                            match z {
                                MarkdownTransformation::MarkdownEventTable(x) => Ok(x),
                                MarkdownTransformation::OptionalMarkdownEvent(Some(x)) => {
                                    Ok(vec![x])
                                }
                                MarkdownTransformation::OptionalMarkdownEvent(None) => {
                                    Ok(Default::default())
                                }
                            }
                        },
                    ),
                    None => crate::markdown::parse_markdown_lua(markdown, |v| Ok(vec![v])),
                },
                lua,
            )
        })?;
        Ok(())
    }
}

pub(crate) fn run_from_walker(
    mut paths: Paths,
    type_defs: TypeWalker,
) -> Result<(), anyhow::Error> {
    let write_path = Path::new(&paths.build_dir).join(&paths.root);
    create_dir_all(&write_path)?;
    let definition_file_storage =
        create_d_file(type_defs.clone(), write_path.clone(), &paths.name, &paths)?;
    let lua_addon_storage = create_lua_addon(
        paths.lua_addon.unwrap_or(LuaAddon::False),
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

    let sidebar: Vec<SideBar> = type_defs
        .given_types
        .iter()
        .map(|v| {
            let members = match v {
                tealr::TypeGenerator::Record(x) => x
                    .functions
                    .iter()
                    .map(|x| x.name.clone())
                    .chain(x.mut_functions.iter().map(|x| x.name.clone()))
                    .chain(x.fields.iter().map(|x| x.name.clone()))
                    .chain(x.static_fields.iter().map(|x| x.name.clone()))
                    .chain(x.methods.iter().map(|x| x.name.clone()))
                    .chain(x.mut_methods.iter().map(|x| x.name.clone()))
                    .chain(x.meta_function.iter().map(|x| x.name.clone()))
                    .chain(x.meta_function_mut.iter().map(|x| x.name.clone()))
                    .chain(x.meta_method.iter().map(|x| x.name.clone()))
                    .chain(x.meta_method_mut.iter().map(|x| x.name.clone()))
                    .map(|x| Members { name: x })
                    .collect(),
                tealr::TypeGenerator::Enum(x) => x
                    .variants
                    .iter()
                    .map(|v| Members { name: v.clone() })
                    .collect(),
            };
            let (name, link_to) = if type_should_be_inlined(v) {
                (paths.name.to_string(), "index".to_string())
            } else {
                let x = get_type_name(v).to_string();
                (x.clone(), x)
            };
            let link_to = sanitize_filename::sanitize(link_to) + ".html";
            SideBar {
                link_to: link_path.join(link_to).to_string_lossy().into_owned(),
                name,
                members,
            }
        })
        .collect();
    let mut z = RecordGenerator::new::<RecordGenerator>(true);

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

struct CreateGlobalInstanceDocs {
    etlua: String,
    template: String,
    type_name: Cow<'static, str>,
}

fn create_globals_docs(
    template_kind: &TemplateKind,
    type_def: &TypeGenerator,
    a: impl FnOnce(CreateGlobalInstanceDocs) -> GlobalInstancesDoc,
) -> Result<(String, GlobalInstancesDoc), anyhow::Error> {
    let type_name = if type_should_be_inlined(type_def) {
        "index".into()
    } else {
        get_type_name(type_def)
    };
    let base_template = include_str!("../base_template.etlua");
    let base_runner = include_str!("../base_run_template.lua");
    let (template, template_runner) = match template_kind {
        TemplateKind::Builtin => (base_template.to_string(), base_runner.to_string()),
        TemplateKind::FromLua(x) => (
            base_template.to_string(),
            std::fs::read_to_string(x)
                .with_context(|| format!("Could not load doc template runner. File {x}"))?,
        ),
        TemplateKind::FromTemplate(x) => (
            std::fs::read_to_string(x)
                .with_context(|| format!("Could not load doc template. File {x}"))?,
            base_runner.to_string(),
        ),
    };
    let etlua = include_str!("../etlua.lua").to_string();
    Ok((
        template_runner,
        a(CreateGlobalInstanceDocs {
            etlua,
            template,
            type_name,
        }),
    ))
}

fn run_and_write(
    write_path: &Path,
    template_runner: &str,
    instance_setter: GlobalInstancesDoc,
) -> Result<(), anyhow::Error> {
    let lua = unsafe { mlu::mlua::Lua::unsafe_new() };

    let file_name = sanitize_filename::sanitize(match &instance_setter.page {
        TypeOrPage::Type(x) => &x.type_name,
        TypeOrPage::IndexPage(x) => &x.type_name,
        TypeOrPage::CustomPage(x) => &x.name,
    })
    .to_owned()
        + ".html";
    mlu::set_global_env(instance_setter, &lua).context("Failed while setting globals")?;

    let document: mlu::mlua::String = lua
        .load(template_runner)
        .set_name("template_runner")
        .call(())
        .context("Failed while running template")?;
    let page_path = write_path.join(file_name);
    let as_bytes = document.as_bytes();
    let mut minify_cfg = minify_html::Cfg::spec_compliant();
    minify_cfg.minify_css = true;
    minify_cfg.minify_js = true;
    let minified = minify_html::minify(as_bytes, &minify_cfg);
    std::fs::write(&page_path, minified)
        .with_context(|| format!("Could not write to {page_path:?}"))?;
    Ok(())
}

#[derive(Default)]
struct GlobalsDefFile {
    etlua: String,
    module: TypeWalker,
    template: String,
    is_global: bool,
    name: String,
}
impl ExportInstances for GlobalsDefFile {
    fn add_instances<'lua, T: mlu::InstanceCollector<'lua>>(
        self,
        instance_collector: &mut T,
    ) -> mlu::mlua::Result<()> {
        instance_collector.document_instance("Type and documentation of this module");
        instance_collector.add_instance("module", |_| Ok(self.module))?;
        instance_collector.document_instance("function to load etlua");
        instance_collector.add_instance("etlua", |lua| {
            lua.load(&self.etlua).set_name("etlua").into_function()
        })?;
        instance_collector
            .document_instance("Wether it is a local or global record that wraps the module");
        instance_collector.add_instance("global_or_local", |_| {
            Ok(if self.is_global { "global" } else { "local" })
        })?;
        instance_collector.document_instance("template that gets loaded");
        instance_collector.add_instance("template", |_| Ok(self.template))?;
        instance_collector.document_instance("name of the module");
        instance_collector.add_instance("name", |_| Ok(self.name))?;
        Ok(())
    }
}

pub(crate) fn create_d_file(
    walker: TypeWalker,
    path: PathBuf,
    name: &str,
    config: &Paths,
) -> Result<PathBuf, anyhow::Error> {
    let is_global = config.is_global;
    let runner = match &config.def_config.runner {
        crate::app::DefTemplateRunnerKind::Builtin => {
            include_str!("../base_run_template.lua").to_string()
        }
        crate::app::DefTemplateRunnerKind::Custom(x) => std::fs::read_to_string(x)
            .with_context(|| format!("Failed loading custom runner: {x}"))?,
    };
    let page_path = path.join("definitions");
    for config in config.def_config.templates.values() {
        let template = match &config.template {
            crate::app::DefTemplateKind::Teal => {
                include_str!("../base_teal_definition_template.etlua").to_string()
            }
            crate::app::DefTemplateKind::Custom(x) => std::fs::read_to_string(x)
                .with_context(|| format!("Failed to load custom template: {x}"))?,
        };
        let lua = unsafe { mlu::mlua::Lua::unsafe_new() };
        let etlua = include_str!("../etlua.lua").to_string();
        let x = GlobalsDefFile {
            etlua,
            module: walker.clone(),
            template,
            is_global,
            name: name.to_string(),
        };
        mlu::set_global_env(x, &lua)?;

        let document: mlu::mlua::String = lua
            .load(&runner)
            .set_name("template_runner")
            .call(())
            .context("Failed running lua template")?;
        create_dir_all(&page_path).with_context(|| {
            format!(
                "Could not create directories needed for:{}.",
                page_path.to_string_lossy()
            )
        })?;
        let extension = if config.extension.starts_with('.') {
            &config.extension[1..]
        } else {
            &config.extension
        };
        let page_path = page_path.join(format!("{name}.{extension}"));
        std::fs::write(&page_path, document.as_bytes())
            .with_context(|| format!("Could not write file{}", page_path.to_string_lossy()))?;
    }

    Ok(page_path)
}

pub fn generate_self_doc() -> Result<TypeWalker, anyhow::Error> {
    use crate::markdown::*;
    let x = tealr::TypeWalker::new()
        .process_type::<tealr::FunctionParam>()
        .process_type::<tealr::FunctionRepresentation>()
        .process_type::<tealr::MapRepresentation>()
        .process_type::<tealr::SingleType>()
        .process_type::<tealr::Name>()
        .process_type::<tealr::ExtraPage>()
        .process_type::<tealr::Type>()
        .process_type::<tealr::EnumGenerator>()
        .process_type::<tealr::ExportedFunction>()
        .process_type::<tealr::Field>()
        .process_type::<tealr::GlobalInstance>()
        .process_type::<tealr::KindOfType>()
        .process_type::<tealr::NamePart>()
        .process_type::<tealr::RecordGenerator>()
        .process_type::<tealr::TealType>()
        .process_type::<tealr::TypeGenerator>()
        .process_type::<tealr::TypeWalker>()
        .process_type::<Members>()
        .process_type::<SideBar>()
        .process_type::<DefTemplateConfig>()
        .process_type::<DefTemplateKind>()
        .process_type::<MarkdownEvent>()
        .process_type::<MarkdownEventCreator>()
        .process_type::<MarkdownCodeBlockKind>()
        .process_type::<MarkdownCodeBlockKindCreator>()
        .process_type::<MarkdownHeadingLevel>()
        .process_type::<MarkdownLinkType>()
        .process_type::<MarkdownTag>()
        .process_type::<MarkdownTagCreator>()
        .process_type::<MarkdownAlignment>()
        .process_type::<crate::find_uses::User>()
        .process_type::<IndexPage>()
        .process_type::<TypeDesc>()
        .process_type::<CustomPage>()
        .process_type::<TypeOrPage>()
        .document_global_instance::<GlobalInstancesDoc>()?;
    Ok(x)
}

pub fn generate_self_def() -> Result<TypeWalker, anyhow::Error> {
    let x = tealr::TypeWalker::new()
        .process_type::<tealr::FunctionParam>()
        .process_type::<tealr::FunctionRepresentation>()
        .process_type::<tealr::MapRepresentation>()
        .process_type::<tealr::SingleType>()
        .process_type::<tealr::Name>()
        .process_type::<tealr::ExtraPage>()
        .process_type::<tealr::Type>()
        .process_type::<tealr::EnumGenerator>()
        .process_type::<tealr::ExportedFunction>()
        .process_type::<tealr::Field>()
        .process_type::<tealr::GlobalInstance>()
        .process_type::<tealr::KindOfType>()
        .process_type::<tealr::NamePart>()
        .process_type::<tealr::RecordGenerator>()
        .process_type::<tealr::TealType>()
        .process_type::<tealr::TypeGenerator>()
        .process_type::<tealr::TypeWalker>()
        .document_global_instance::<GlobalsDefFile>()?;
    Ok(x)
}
