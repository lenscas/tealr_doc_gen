use app::Paths;
use generation::{generate_self_def, generate_self_doc, run_from_walker, run_template};

use crate::app::get_paths;

mod app;
mod credits;
mod doc_gen;
mod find_uses;
mod generation;
mod markdown;
mod render_type;
fn main() -> anyhow::Result<()> {
    match get_paths()? {
        app::Modes::Credits => {
            credits::show_credits();
        }
        app::Modes::GenerateDocs(x) => {
            //generate_docs::generate_docs(x)
            run_template(*x)?;
        }
        app::Modes::SelfDocTemplate { build_dir } => {
            let walker = generate_self_doc()?;
            run_from_walker(
                Paths {
                    is_global: true,
                    json: "{}".into(),
                    name: "tealr_doc_gen".into(),
                    root: "".into(),
                    build_dir,
                    template_kind: app::TemplateKind::Builtin,
                    def_config: Default::default(),
                    lua_addon: Some(app::LuaAddon::Create {
                        words: Default::default(),
                        files: Default::default(),
                        settings: Default::default(),
                    }),
                },
                walker,
            )?;
        }

        app::Modes::GenFile { file, location } => {
            //std::fs::create_dir_all(location)?;
            std::fs::write(location, file)?;
        }
        app::Modes::Nothing => (),
        app::Modes::SelfDefTemplate { build_dir } => {
            let walker = generate_self_def()?;
            run_from_walker(
                Paths {
                    is_global: true,
                    json: "{}".into(),
                    name: "tealr_doc_gen".into(),
                    root: "".into(),
                    build_dir,
                    template_kind: app::TemplateKind::Builtin,
                    def_config: Default::default(),
                    lua_addon: Some(app::LuaAddon::Create {
                        words: Default::default(),
                        files: Default::default(),
                        settings: Default::default(),
                    }),
                },
                walker,
            )?;
        }
    }
    Ok(())
}
