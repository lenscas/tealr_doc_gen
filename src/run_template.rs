use std::{
    collections::HashMap,
    fs::{create_dir_all, read_to_string},
    path::{Path, PathBuf},
};

use anyhow::Context;
use tealr::{
    mlu::{self, ExportInstances, FromToLua, TealData, TypedFunction},
    EnumGenerator, GlobalInstance, NameContainer, RecordGenerator, TypeGenerator, TypeName,
    TypeWalker,
};

use crate::{
    app::{DefTemplateConfig, DefTemplateKind, Paths, TemplateKind},
    doc_gen::{get_type_name, type_should_be_inlined},
    markdown::MarkdownEvent,
};

#[derive(FromToLua, TypeName, Clone)]
/// An element in the sidebar
struct SideBar {
    /// What url to link to
    link_to: String,
    /// Name of the type in the sidebar
    name: String,
    /// The members of the type
    members: Vec<Members>,
}
#[derive(FromToLua, TypeName, Clone)]
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
struct GlobalInstancesDoc {
    side_bar: Vec<SideBar>,
    link_path: PathBuf,
    etlua: String,
    template: String,
    type_members: TypeGenerator,
    type_name: String,
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
            type_members: TypeGenerator::Enum(EnumGenerator {
                name: Default::default(),
                variants: Default::default(),
                type_doc: Default::default(),
            }),
            type_name: Default::default(),
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
        let type_members = self.type_members;
        let type_name = self.type_name;
        let all_types = self.all_types;
        let globals = self.globals;
        let definition_files_folder = self.definition_files_folder;

        instance_collector.add_instance("side_bar_types", move |_| Ok(side_bar))?;
        instance_collector.add_instance("etlua", move |lua| {
            lua.load(&etlua).set_name("etlua")?.into_function()
        })?;
        instance_collector.add_instance("template", move |_| Ok(template))?;
        instance_collector.add_instance("type_name", move |_| Ok(type_name))?;
        instance_collector.add_instance("type_members", move |_| Ok(type_members))?;
        instance_collector.add_instance("globals", move |_| Ok(globals))?;
        instance_collector.add_instance("all_types", move |_| Ok(all_types))?;
        instance_collector.add_instance("create_link", move |lua| {
            let link = link_path;
            TypedFunction::from_rust(
                move |_, name: String| Ok(link.join(name + ".html").to_string_lossy().to_string()),
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

pub(crate) fn run_from_walker(paths: Paths, type_defs: TypeWalker) -> Result<(), anyhow::Error> {
    let write_path = Path::new(&paths.build_dir).join(&paths.root);
    create_dir_all(&write_path)?;
    let definition_file_storage =
        create_d_file(type_defs.clone(), write_path.clone(), &paths.name, &paths)?;
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
            let link_to = link_to + ".html";
            SideBar {
                link_to: link_path.join(link_to).to_string_lossy().into_owned(),
                name,
                members,
            }
        })
        .collect();
    let mut z = RecordGenerator::new::<RecordGenerator>(true);
    for type_def in type_defs.iter() {
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
        run_and_write(
            sidebar.clone(),
            type_def,
            &write_path,
            None,
            None,
            link_path.clone(),
            &paths.template_kind,
            paths.def_config.templates.clone(),
            paths.name.clone(),
            definition_file_storage.clone(),
        )?;
    }
    run_and_write(
        sidebar,
        &TypeGenerator::Record(Box::new(z)),
        &write_path,
        Some(type_defs.global_instances_off),
        Some(type_defs.given_types),
        link_path,
        &paths.template_kind,
        paths.def_config.templates,
        paths.name,
        definition_file_storage,
    )?;
    Ok(())
}

pub(crate) fn run_template(paths: Paths) -> Result<(), anyhow::Error> {
    let json = read_to_string(&paths.json)?;
    let type_defs: tealr::TypeWalker = serde_json::from_str(&json)?;
    run_from_walker(paths, type_defs)
}

fn run_and_write(
    sidebar: Vec<SideBar>,
    type_def: &TypeGenerator,
    write_path: &Path,
    global_instances: Option<Vec<tealr::GlobalInstance>>,
    all_types: Option<Vec<TypeGenerator>>,
    link_path: PathBuf,
    template_kind: &TemplateKind,
    def_config: HashMap<String, DefTemplateConfig>,
    library_name: String,
    definition_files_storage: PathBuf,
) -> Result<(), anyhow::Error> {
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
    let lua = unsafe { mlu::mlua::Lua::unsafe_new() };
    let instance_setter = GlobalInstancesDoc {
        side_bar: sidebar,
        link_path,
        etlua,
        template,
        type_members: type_def.to_owned(),
        type_name: type_name.to_string(),
        all_types,
        globals: global_instances,
        def_files: def_config,
        library_name,
        definition_files_folder: definition_files_storage.to_string_lossy().to_string(),
    };
    mlu::set_global_env(instance_setter, &lua)?;

    let document: mlu::mlua::String = lua
        .load(&template_runner)
        .set_name("template_runner")?
        .call(())?;
    let page_path = write_path.join(format!("{type_name}.html"));
    std::fs::write(page_path, document.as_bytes())?;
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
            lua.load(&self.etlua).set_name("etlua")?.into_function()
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
    let runner = match &config.def_config.runner {
        crate::app::DefTemplateRunnerKind::Builtin => {
            include_str!("../base_run_template.lua").to_string()
        }
        crate::app::DefTemplateRunnerKind::Custom(x) => std::fs::read_to_string(&x)
            .with_context(|| format!("Failed loading custom runner: {x}"))?,
    };
    let page_path = path.join("definitions");
    for config in config.def_config.templates.values() {
        let template = match &config.template {
            crate::app::DefTemplateKind::Teal => {
                include_str!("../base_teal_definition_template.etlua").to_string()
            }
            crate::app::DefTemplateKind::Custom(x) => std::fs::read_to_string(&x)
                .with_context(|| format!("Failed to load custom template: {x}"))?,
        };
        let lua = unsafe { mlu::mlua::Lua::unsafe_new() };
        let etlua = include_str!("../etlua.lua").to_string();
        let x = GlobalsDefFile {
            etlua,
            module: walker.clone(),
            template,
            is_global: true,
            name: name.to_string(),
        };
        mlu::set_global_env(x, &lua)?;

        let document: mlu::mlua::String = lua.load(&runner).set_name("template_runner")?.call(())?;
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
        .document_global_instance::<GlobalInstancesDoc>()?;
    Ok(x)
}

pub fn generate_self_def() -> Result<TypeWalker, anyhow::Error> {
    let x = tealr::TypeWalker::new()
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
