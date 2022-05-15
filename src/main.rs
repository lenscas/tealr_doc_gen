use std::{
    fs::{create_dir_all, read_to_string},
    path::Path,
};

use doc_gen::get_type_name;

use crate::{
    app::get_paths,
    doc_gen::{gen_doc, gen_line, gen_side_bar, gen_type_page, get_type},
};

mod app;
mod compile_teal;
mod doc_gen;
mod markdown;
fn main() -> anyhow::Result<()> {
    let paths = get_paths();
    let json = read_to_string(paths.json)?;
    let type_defs: tealr::TypeWalker = serde_json::from_str(&json)?;

    let mut containers = String::new();
    containers += &type_defs
        .iter()
        .filter(|v| match v {
            tealr::TypeGenerator::Record(x) => x.should_be_inlined,
            tealr::TypeGenerator::Enum(_) => false,
        })
        .map(get_type)
        .collect::<String>();
    containers += "<div class=\"panel\">
            <div class=\"panel-heading\">
                <p class=\"panel-header-title\">Types:</p>
            </div>
            <div class=\"panel-block\">
                <div class=\"container\">
        ";
    let write_path = Path::new(&paths.build_dir).join(&paths.root);
    let link_path = Path::new("").join(&paths.root);
    create_dir_all(&write_path)?;
    for type_def in type_defs.iter().filter(|v| match v {
        tealr::TypeGenerator::Record(x) => !x.should_be_inlined,
        tealr::TypeGenerator::Enum(_) => true,
    }) {
        let side_bar = gen_side_bar(&type_defs, Some(type_def), &paths.name, &link_path);
        let page = gen_type_page(type_def, side_bar);
        let type_name = get_type_name(type_def);
        let page_path = write_path.join(format!("{type_name}.html"));
        std::fs::write(page_path, page)?;
        containers += &gen_line(type_def, &link_path);
    }
    containers += "</div></div>";
    let side_bar = gen_side_bar(&type_defs, None, &paths.name, &link_path);
    let document = gen_doc(&containers, &paths.name, side_bar);
    let path = write_path.join("index.html");
    std::fs::write(path, document)?;
    Ok(())
}
