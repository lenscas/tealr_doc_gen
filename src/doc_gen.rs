use std::path::Path;

use tealr::{ExportedFunction, NameContainer, TealType, TypeGenerator, TypeWalker};

use crate::markdown::parse_markdown;

const LEFT_BRACKET_ESCAPED: &str = "&lt;";
const RIGHT_BRACKET_ESCAPED: &str = "&gt;";

fn gen_type_name(v: &str) -> String {
    format!("<div class=\"card-header\"><code class=\"card-header-title\">{v}</code></div>")
}
fn gen_container(contents: &str, link_to: &Path) -> String {
    let link_to = link_to.to_string_lossy();
    format!("<div class=\"card block\"><a href=\"{link_to}.html\">{contents}</a></div>")
    //format!("<div class=\"card block\">{contents}</div>")
}
pub(crate) fn gen_doc(contents: &str, title: &str, side_bar: String) -> String {
    let x = include_str!("../base_html.html").to_string();
    let body = format!("<div class=\"container\">\n{contents}\n</div>");
    x.replace("{BODY_HERE}", &body)
        .replace("{TITLE_HERE}", title)
        .replace("{SIDE_BAR_HERE}", &side_bar)
}
pub(crate) fn gen_line(type_def: &TypeGenerator, path: &Path) -> String {
    let raw_name = tealr::type_parts_to_str(type_def.type_name.clone());
    let name = gen_type_name(&raw_name);
    let doc = parse_markdown(&type_def.type_doc);
    let link_to = path.join("raw_name");
    gen_container(
        &format!("{name} <div class=\"card-content content\">{doc}</div>"),
        &link_to,
    )
}
fn get_doc(type_def: &TypeGenerator, name: &NameContainer) -> String {
    type_def
        .documentation
        .get(name)
        .map(|v| parse_markdown(v))
        .map(|v| format!("<div class=\"card-content content\">{v}</div>"))
        .unwrap_or_default()
}

fn gen_type(type_def: &TealType, should_recurse: bool) -> String {
    let name = &type_def.name;
    let x = Vec::new();
    let generics = type_def
        .generics
        .as_ref()
        .unwrap_or(&x)
        .iter()
        .filter(|x| x.type_kind == tealr::KindOfType::Generic)
        .map(|v| gen_type(v, should_recurse))
        .collect::<Vec<_>>()
        .join(",");
    let generics = if generics.is_empty() || !should_recurse {
        "".to_string()
    } else {
        format!("{LEFT_BRACKET_ESCAPED}{generics}{RIGHT_BRACKET_ESCAPED}")
    };
    match type_def.type_kind {
        tealr::KindOfType::Builtin | tealr::KindOfType::Generic => format!("{name}{generics}"),
        tealr::KindOfType::External => format!("<a href=\"{name}.html\">{name}</a>{generics}"),
    }
}

fn create_methods_iter<'a>(
    type_def: &'a TypeGenerator,
) -> impl Iterator<Item = (bool, &ExportedFunction)> + 'a {
    type_def
        .methods
        .iter()
        .map(|v| (true, v))
        .chain(type_def.mut_methods.iter().map(|v| (true, v)))
        .chain(type_def.functions.iter().map(|v| (false, v)))
        .chain(type_def.mut_functions.iter().map(|v| (false, v)))
        .chain(type_def.meta_function.iter().map(|v| (false, v)))
        .chain(type_def.meta_function_mut.iter().map(|v| (false, v)))
        .chain(type_def.meta_method.iter().map(|v| (true, v)))
        .chain(type_def.meta_method_mut.iter().map(|v| (true, v)))
}

pub fn get_type(type_def: &TypeGenerator) -> String {
    let fields = type_def
        .fields
        .iter()
        .map(|(name, written_type)| {
            let doc = get_doc(type_def, &NameContainer::from(name.clone()));
            format!(
                "
                    <div class=\"card block\">
                        <div class=\"card-heading\" id={name}>
                            <code class=\"card-header-title\"><p>{name}: {written_type}</p></code>
                        </div>
                        {doc}
                    </div>
                "
            )
        })
        .collect::<String>();
    let methods = create_methods_iter(type_def)
        .map(|(_, v)| {
            let method_name = String::from_utf8_lossy(&v.name);
            let signature = {
                let signature = v
                    .signature
                    .iter()
                    .map(|v| match v {
                        tealr::NamePart::Symbol(x) => v_htmlescape::escape(x).to_string(),
                        tealr::NamePart::Type(x) => gen_type(x, true),
                    })
                    .collect::<String>();
                let meta_method = if v.is_meta_method { "metamethod" } else { "" };
                let method_name = String::from_utf8_lossy(&v.name);
                format!("{meta_method} {method_name}: {signature}")
            };
            let documentation = get_doc(type_def, &v.name);
            format!(
                "
                    <div class=\"card block\">
                        <div class=\"card-heading\" id=\"{method_name}\">
                            <code class=\"card-header-title\"><p>{signature}</p></code>
                        </div>
                        {documentation}
                    </div>"
            )
        })
        .collect::<String>();
    let methods = if methods.is_empty() {
        "".to_string()
    } else {
        format!("<div class=\"container\">{methods}</div>")
    };
    let fields = if fields.is_empty() {
        "".to_string()
    } else {
        format!("<div class=\"container\">{fields}</div>")
    };
    let type_doc = if type_def.type_doc.is_empty() {
        "".to_string()
    } else {
        let doc = parse_markdown(&type_def.type_doc);
        format!(
            "<div class=\"panel-block\">
                <div class=\"container\">
                    {doc}
                </div>
            </div>"
        )
    };
    format!(
        "<div class=\"panel block\">
            <div class=\"panel-heading\">
                <p class=\"subtitle\">
                    Type doc:
                </p>
            </div>
            {type_doc}
        </div>
        <div class=\"panel block\">
            <div class=\"panel-heading\">
                <p class=\"subtitle\">
                    Fields:
                </p>
            </div>
            <div class=\"panel-block\">
                {fields}
            </div>

        </div>
        <div class=\"panel block\">
            <div class=\"panel-heading\">
                <p class=\"subtitle\">
                    Methods:
                </p>
            </div>
            <div class=\"panel-block\">
                {methods}
            </div>
        </div>"
    )
}

pub(crate) fn gen_type_page(type_def: &TypeGenerator, side_bar: String) -> String {
    let page = get_type(type_def);
    gen_doc(
        &page,
        &tealr::type_parts_to_str(type_def.type_name.clone()),
        side_bar,
    )
}

pub(crate) fn gen_sidebar_item(link_to: &Path, field_name: &str) -> String {
    let link_to = link_to.to_string_lossy();
    format!(
        "<li>
    <a href=\"{link_to}.html#{field_name}\">
        <span class=\"icon is-small\"><i class=\"fa fa-link\"></i></span> {field_name}
    </a>
</li>"
    )
}

pub(crate) fn gen_sidebar_tab(members: String, member_type: &str) -> String {
    if members.is_empty() {
        "".to_string()
    } else {
        format!(
            "<li>{member_type}</li>
            {members}"
        )
    }
}

pub(crate) fn gen_side_bar(
    all_types: &TypeWalker,
    current: Option<&TypeGenerator>,
    index: &str,
    path: &Path,
) -> String {
    (all_types.iter().filter(|v| v.should_be_inlined))
        .chain(all_types.iter().filter(|v| !v.should_be_inlined))
        .map(|v| {
            let (name, link_to) = if v.should_be_inlined {
                (index.to_string(), "index".to_string())
            } else {
                let x = tealr::type_parts_to_str(v.type_name.clone()).to_string();
                (x.clone(), x)
            };
            let link_to = path.join(link_to);
            let is_active = current
                .map(|current| v.type_name == current.type_name)
                .unwrap_or_else(|| v.should_be_inlined)
                .then(|| "is-active")
                .unwrap_or("");
            let fields = v
                .fields
                .iter()
                .map(|(field_name, _)| gen_sidebar_item(&link_to, field_name))
                .collect::<String>();
            let fields = gen_sidebar_tab(fields, "Fields");
            let methods = create_methods_iter(v)
                .map(|(_, v)| v)
                .map(|v| gen_sidebar_item(&link_to, &String::from_utf8_lossy(&v.name)))
                .collect::<String>();
            let methods = gen_sidebar_tab(methods, "Methods");
            let link_to = link_to.to_string_lossy();
            format!(
                "<li>
                    <a href=\"{link_to}.html\" class=\"{is_active}\">
                        <span class=\"icon\"><i class=\"fa fa-file\"></i></span> {name}
                    </a>
                    <ul>
                       {fields}
                       {methods}
                    </ul>
                </li>"
            )
        })
        .collect::<String>()
}
