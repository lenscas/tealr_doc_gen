use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Context;
use tealr::{
    mlu::{self, ExportInstances, FromToLua, TypedFunction},
    type_to_string, GlobalInstance, ToTypename, TypeGenerator, TypeWalker,
};

use crate::{
    app::{DefTemplateConfig, DefTemplateKind, TemplateKind},
    doc_gen::{get_type_name, type_should_be_inlined},
    find_uses::NameAndSignature,
    generation::{
        shared_globals, shared_types,
        sidebar::{Members, SideBar},
    },
    markdown::MarkdownEvent,
    render_type::{type_to_link_url, SingleTypeNoConsume},
};

type MarkdownEventTable = Vec<MarkdownEvent>;
type OptionalMarkdownEvent = Option<MarkdownEvent>;

tealr::create_union_mlua!(pub enum MarkdownTransformation = MarkdownEventTable | OptionalMarkdownEvent );

#[derive(Clone, FromToLua, ToTypename)]
pub(super) enum TypeOrPage {
    Type(TypeDesc),
    IndexPage(IndexPage),
    CustomPage(CustomPage),
}
#[derive(Clone, FromToLua, ToTypename)]
pub(super) struct TypeDesc {
    pub(super) type_members: TypeGenerator,
    pub(super) type_name: String,
    pub(super) used_by: Vec<crate::find_uses::User>,
}
#[derive(Clone, FromToLua, ToTypename)]
pub(super) struct IndexPage {
    pub(super) all_types: Vec<TypeGenerator>,
    pub(super) type_name: String,
    pub(super) type_members: TypeGenerator,
}
#[derive(Clone, FromToLua, ToTypename)]
pub(super) struct CustomPage {
    pub(super) name: String,
    pub(super) markdown_content: String,
}

#[derive(Clone)]
pub(super) struct GlobalInstancesDoc {
    pub(super) side_bar: Vec<SideBar>,
    pub(super) link_path: PathBuf,
    pub(super) etlua: String,
    pub(super) template: String,
    pub(super) page: TypeOrPage,
    pub(super) all_types: Option<Vec<TypeGenerator>>,
    pub(super) globals: Option<Vec<GlobalInstance>>,
    pub(super) def_files: HashMap<String, DefTemplateConfig>,
    pub(super) library_name: String,
    pub(super) definition_files_folder: String,
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

impl ExportInstances for GlobalInstancesDoc {
    fn add_instances<T: mlu::InstanceCollector>(
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
        instance_collector.add_instance("page", move |_| Ok(self.page))?;
        instance_collector.add_instance("globals", move |_| Ok(globals))?;
        let all_types_2 = all_types.clone();
        instance_collector.add_instance("all_types", move |_| Ok(all_types_2))?;
        let link_path2 = link_path.clone();
        let all_types2 = all_types.clone();
        instance_collector.add_instance("type_to_link", move |lua| {
            let link = link_path2;
            let all_types = all_types2;
            TypedFunction::from_rust(
                move |_, ty: SingleTypeNoConsume| {
                    let all_types = all_types.clone();
                    let link = link.clone();
                    Ok(type_to_link_url(ty, &all_types.unwrap_or_default(), link))
                },
                lua,
            )
        })?;
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
                                x.should_be_inlined && type_to_string(&x.ty, false) == name
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
        instance_collector.add_instance("markdown_tag_end_creator", |_| {
            Ok(crate::markdown::MarkdownTagEndCreator {})
        })?;

        instance_collector.add_instance("library_name", |_| Ok(self.library_name))?;
        instance_collector.add_instance("definition_config", |_| Ok(self.def_files))?;

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
        shared_globals(instance_collector, &etlua, &template)?;
        Ok(())
    }
}

pub(super) struct CreateGlobalInstanceDocs {
    pub(super) etlua: String,
    pub(super) template: String,
    pub(super) type_name: Cow<'static, str>,
}

pub(super) fn create_globals_docs(
    template_kind: &TemplateKind,
    type_def: &TypeGenerator,
    a: impl FnOnce(CreateGlobalInstanceDocs) -> GlobalInstancesDoc,
) -> Result<(String, GlobalInstancesDoc), anyhow::Error> {
    let type_name = if type_should_be_inlined(type_def) {
        "index".into()
    } else {
        get_type_name(type_def)
    };
    let base_template = include_str!("../../base_template.etlua");
    let base_runner = include_str!("../../base_run_template.lua");
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
    let etlua = include_str!("../../etlua.lua").to_string();
    Ok((
        template_runner,
        a(CreateGlobalInstanceDocs {
            etlua,
            template,
            type_name: type_name.into(),
        }),
    ))
}

pub(super) fn run_and_write(
    write_path: &Path,
    template_runner: &str,
    instance_setter: GlobalInstancesDoc,
) -> Result<(), anyhow::Error> {
    let lua = unsafe { mlu::mlua::Lua::unsafe_new() };
    let page_path: PathBuf = match &instance_setter.page {
        TypeOrPage::Type(x) => match x.type_members.type_name() {
            tealr::Type::Single(single_type) => type_to_link_url(
                single_type.to_owned(),
                instance_setter.all_types.as_deref().unwrap_or_default(),
                write_path.to_owned(),
            )
            .unwrap_or_else(|| write_path.join(sanitize_filename::sanitize(&x.type_name))),
            tealr::Type::Function(_)
            | tealr::Type::Map(_)
            | tealr::Type::Or(_)
            | tealr::Type::Array(_)
            | tealr::Type::Tuple(_)
            | tealr::Type::Variadic(_) => {
                write_path.join(sanitize_filename::sanitize(&x.type_name))
            }
        },
        TypeOrPage::IndexPage(x) => write_path.join(sanitize_filename::sanitize(&x.type_name)),
        TypeOrPage::CustomPage(x) => write_path.join(sanitize_filename::sanitize(&x.name)),
    }
    .with_extension("html");
    mlu::set_global_env(instance_setter, &lua).context("Failed while setting globals")?;

    let document: mlu::mlua::String = lua
        .load(template_runner)
        .set_name("template_runner")
        .call(())
        .context("Failed while running template")?;
    let as_bytes = document.as_bytes();
    let mut minify_cfg = minify_html::Cfg::spec_compliant();
    minify_cfg.minify_css = true;
    //there are currently bugs when minifying js (https://github.com/wilsonzlin/minify-js/issues/15)
    minify_cfg.minify_js = false;
    let minified = minify_html::minify(&as_bytes, &minify_cfg);
    std::fs::write(&page_path, minified)
        .with_context(|| format!("Could not write to {page_path:?}"))?;
    Ok(())
}

pub fn generate_self_doc() -> Result<TypeWalker, anyhow::Error> {
    use crate::markdown::*;
    shared_types(tealr::TypeWalker::new())
        .process_type::<NameAndSignature>()
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
        .process_type::<MarkdownMetadataBlockKind>()
        .process_type::<MarkdownTagEnd>()
        .process_type::<MarkdownBlockQuoteKind>()
        .process_type::<MarkdownAttribute>()
        .process_type::<MarkdownTagEndCreator>()
        .process_type::<crate::find_uses::User>()
        .process_type::<IndexPage>()
        .process_type::<TypeDesc>()
        .process_type::<CustomPage>()
        .process_type::<TypeOrPage>()
        .document_global_instance::<GlobalInstancesDoc>()
        .context("failed to generate global instance")
}
