use std::{
    fs::{create_dir_all, read_to_string},
    path::{Path, PathBuf},
};

use anyhow::Context;
use tealr::{
    mlu::{ExportInstances, FromToLua, TealData},
    EnumGenerator, GlobalInstance, NameContainer, RecordGenerator, TypeGenerator, TypeName,
    TypeWalker,
};

use crate::{
    app::{Paths, TemplateKind},
    doc_gen::{get_type_name, type_should_be_inlined},
};

#[derive(FromToLua, TypeName, Clone)]
struct SideBar {
    link_to: String,
    name: String,
    members: Vec<Members>,
}
#[derive(FromToLua, TypeName, Clone)]
struct Members {
    name: NameContainer,
}

#[derive(tealr::mlu::TealDerive)]
struct TestDocs;
impl TealData for TestDocs {
    fn add_methods<'lua, T: tealr::mlu::TealDataMethods<'lua, Self>>(methods: &mut T) {
        methods.document("```teal_lua");
        methods.document("local a:number = 1");
        methods.document("```");
        methods.document("some documentation for the next method");
        methods.add_method("test", |_, _, ()| Ok(1));
        methods.document_type("some documentation for the entire type");
    }

    fn add_fields<'lua, F: tealr::mlu::TealDataFields<'lua, Self>>(fields: &mut F) {
        fields.document("some docs for the next field");
        fields.add_field_function_get("a", |_, _| Ok(1))
    }
}
struct GlobalInstances {
    side_bar: Vec<SideBar>,
    link_path: PathBuf,
    etlua: String,
    template: String,
    type_members: TypeGenerator,
    type_name: String,
    all_types: Option<Vec<TypeGenerator>>,
    globals: Option<Vec<GlobalInstance>>,
}
impl Default for GlobalInstances {
    fn default() -> Self {
        Self {
            side_bar: Default::default(),
            link_path: Default::default(),
            etlua: Default::default(),
            template: Default::default(),
            type_members: TypeGenerator::Enum(EnumGenerator {
                name: Default::default(),
                variants: Default::default(),
            }),
            type_name: Default::default(),
            all_types: Default::default(),
            globals: Default::default(),
        }
    }
}

impl ExportInstances for GlobalInstances {
    fn add_instances<'lua, T: tealr::mlu::InstanceCollector<'lua>>(
        self,
        instance_collector: &mut T,
    ) -> tealr::mlu::mlua::Result<()> {
        let side_bar = self.side_bar;
        let link_path = self.link_path;
        let etlua = self.etlua;
        let template = self.template;
        let type_members = self.type_members;
        let type_name = self.type_name;
        let all_types = self.all_types;
        let globals = self.globals;

        instance_collector.add_instance("side_bar_types".into(), move |_| Ok(side_bar))?;
        instance_collector.add_instance("etlua".into(), move |lua| {
            lua.load(&etlua).set_name("etlua")?.into_function()
        })?;
        instance_collector.add_instance("template".into(), move |_| Ok(template))?;
        instance_collector.add_instance("type_name".into(), move |_| Ok(type_name))?;
        instance_collector.add_instance("type_members".into(), move |_| Ok(type_members))?;
        instance_collector.add_instance("globals".into(), move |_| Ok(globals))?;
        instance_collector.add_instance("all_types".into(), move |_| Ok(all_types))?;
        instance_collector.add_instance("create_link".into(), move |lua| {
            let link = link_path;
            tealr::mlu::TypedFunction::from_rust(
                move |_, name: String| Ok(link.join(name + ".html").to_string_lossy().to_string()),
                lua,
            )
        })?;
        instance_collector.add_instance("dedupe_by".into(), |lua| {
            tealr::mlu::TypedFunction::from_rust(
                |lua,
                 (a, b): (
                    Vec<tealr::mlu::generics::X>,
                    tealr::mlu::TypedFunction<tealr::mlu::generics::X, tealr::mlu::generics::Y>,
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
        instance_collector.add_instance("parse_markdown".into(), |lua| {
            tealr::mlu::TypedFunction::from_rust(
                |_, markdown: String| Ok(crate::markdown::parse_markdown(&markdown)),
                lua,
            )
        })?;
        Ok(())
    }
}

pub(crate) fn run_from_walker(paths: Paths, type_defs: TypeWalker) -> Result<(), anyhow::Error> {
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
    let write_path = Path::new(&paths.build_dir).join(&paths.root);
    create_dir_all(&write_path)?;
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
    let lua = unsafe { tealr::mlu::mlua::Lua::unsafe_new() };
    let instance_setter = GlobalInstances {
        side_bar: sidebar,
        link_path,
        etlua,
        template,
        type_members: type_def.to_owned(),
        type_name: type_name.to_string(),
        all_types,
        globals: global_instances,
    };
    tealr::mlu::set_global_env(instance_setter, &lua)?;

    let document: tealr::mlu::mlua::String = lua
        .load(&template_runner)
        .set_name("template_runner")?
        .call(())?;
    let page_path = write_path.join(format!("{type_name}.html"));
    std::fs::write(page_path, document.as_bytes())?;
    Ok(())
}

pub fn generate_self() -> Result<TypeWalker, anyhow::Error> {
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
        .process_type::<TestDocs>()
        .process_type_inline::<TestDocs>()
        .document_global_instance::<GlobalInstances>()?;
    Ok(x)
}
