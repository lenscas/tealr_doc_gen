use std::{fs::create_dir_all, path::PathBuf};

use anyhow::Context;
use tealr::{
    mlu::{self, ExportInstances},
    TypeWalker,
};

use crate::Paths;

use super::{shared_globals, shared_types};

#[derive(Default)]
struct GlobalsDefFile {
    etlua: String,
    module: TypeWalker,
    template: String,
    is_global: bool,
    name: String,
}
impl ExportInstances for GlobalsDefFile {
    fn add_instances<T: mlu::InstanceCollector>(
        self,
        instance_collector: &mut T,
    ) -> mlu::mlua::Result<()> {
        instance_collector.document_instance("Type and documentation of this module");
        instance_collector.add_instance("module", |_| Ok(self.module))?;
        instance_collector
            .document_instance("Wether it is a local or global record that wraps the module");
        instance_collector.add_instance("global_or_local", |_| {
            Ok(if self.is_global { "global" } else { "local" })
        })?;

        instance_collector.document_instance("name of the module");
        instance_collector.add_instance("name", |_| Ok(self.name))?;
        shared_globals(instance_collector, &self.etlua, &self.template)?;
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
            include_str!("../../base_run_template.lua").to_string()
        }
        crate::app::DefTemplateRunnerKind::Custom(x) => std::fs::read_to_string(x)
            .with_context(|| format!("Failed loading custom runner: {x}"))?,
    };
    let page_path = path.join("definitions");
    for config in config.def_config.templates.values() {
        let template = match &config.template {
            crate::app::DefTemplateKind::Teal => {
                include_str!("../../base_teal_definition_template.etlua").to_string()
            }
            crate::app::DefTemplateKind::Custom(x) => std::fs::read_to_string(x)
                .with_context(|| format!("Failed to load custom template: {x}"))?,
        };
        let lua = unsafe { mlu::mlua::Lua::unsafe_new() };
        let etlua = include_str!("../../etlua.lua").to_string();
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

pub fn generate_self_def() -> Result<TypeWalker, anyhow::Error> {
    let x = tealr::TypeWalker::new();
    shared_types(x)
        .document_global_instance::<GlobalsDefFile>()
        .context("failed to generate global instance")
}
