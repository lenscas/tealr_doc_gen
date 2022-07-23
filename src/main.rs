use crate::app::get_paths;

mod app;
mod compile_teal;
mod credits;
mod doc_gen;
mod markdown;
mod run_template;
fn main() -> anyhow::Result<()> {
    match get_paths() {
        app::Modes::Credits => {
            credits::show_credits();
            Ok(())
        }
        app::Modes::GenerateDocs(x) => {
            //generate_docs::generate_docs(x)
            run_template::run_template(x)
        }
    }
}
