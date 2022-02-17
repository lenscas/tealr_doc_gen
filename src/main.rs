use std::fs::read_to_string;

use crate::{
    app::get_paths,
    doc_gen::{gen_doc, gen_line, gen_side_bar, gen_type_page, get_type},
};

mod app;
mod compile_teal;
mod doc_gen;
mod markdown;
fn main() {
    let paths = get_paths();
    let json = read_to_string(paths.json).unwrap();
    let type_defs: tealr::TypeWalker = serde_json::from_str(&json).unwrap();

    let mut containers = String::new();
    containers += &type_defs
        .iter()
        .filter(|v| v.should_be_inlined)
        .map(get_type)
        .collect::<String>();
    containers += "<div class=\"panel\">
            <div class=\"panel-heading\">
                <p class=\"panel-header-title\">Types:</p>
            </div>
            <div class=\"panel-block\">
                <div class=\"container\">
        ";
    for type_def in type_defs.iter().filter(|v| !v.should_be_inlined) {
        let side_bar = gen_side_bar(&type_defs, Some(type_def), &paths.name);
        let page = gen_type_page(type_def, side_bar);
        std::fs::write(
            format!(
                "./pages/{}.html",
                tealr::type_parts_to_str(type_def.type_name.clone())
            ),
            page,
        )
        .unwrap();
        containers += &gen_line(type_def);
    }
    containers += "</div></div>";
    let side_bar = gen_side_bar(&type_defs, None, &paths.name);
    let document = gen_doc(&containers, &paths.name, side_bar);
    std::fs::write("./pages/index.html", document).unwrap();
}
