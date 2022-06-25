use std::{
    borrow::Cow,
    collections::HashSet,
    path::{Path, PathBuf},
};

use tealr::{
    EnumGenerator, ExportedFunction, NameContainer, NamePart, RecordGenerator, TealType,
    TypeGenerator, TypeWalker,
};

use crate::markdown::parse_markdown;

const LEFT_BRACKET_ESCAPED: &str = "&lt;";
const RIGHT_BRACKET_ESCAPED: &str = "&gt;";

fn dedupe_by<T, K: Eq + std::hash::Hash + 'static, I: Iterator<Item = T>>(
    iter: I,
    deduper: impl Fn(&T) -> K,
) -> impl Iterator<Item = T> {
    let mut dupes = HashSet::new();
    iter.filter(move |v| dupes.insert(deduper(v)))
}

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

pub(crate) fn get_type_name(type_def: &TypeGenerator) -> Cow<'static, str> {
    let z = match type_def {
        TypeGenerator::Record(x) => &x.type_name,
        TypeGenerator::Enum(x) => &x.name,
    };
    tealr::type_parts_to_str(z.clone())
}

pub(crate) fn gen_line(type_def: &TypeGenerator, path: &Path) -> String {
    let raw_name = get_type_name(type_def);
    let name = gen_type_name(&raw_name);
    let doc = match type_def {
        TypeGenerator::Record(x) => parse_markdown(&x.type_doc),
        TypeGenerator::Enum(_) => String::new(),
    };
    let link_to = path.join(raw_name.as_ref());
    gen_container(
        &format!("{name} <div class=\"card-content content\">{doc}</div>"),
        &link_to,
    )
}
fn get_doc_record(record_def: &RecordGenerator, name: &NameContainer) -> String {
    record_def
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
    type_def: &'a RecordGenerator,
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

fn parse_namepart(part: &[NamePart]) -> String {
    part.iter()
        .map(|v| match v {
            tealr::NamePart::Symbol(x) => v_htmlescape::escape(x).to_string(),
            tealr::NamePart::Type(x) => gen_type(x, true),
        })
        .collect::<String>()
}
pub fn get_enum_type(type_def: &EnumGenerator) -> String {
    let variants: String = type_def
        .variants
        .iter()
        .map(|v| {
            let x = String::from_utf8_lossy(v)
                .replace('\\', "\\\\")
                .replace('"', "\\\"");
            let x = format!("\"{x}\"");
            let name = v_htmlescape::escape(&x);
            format!(
                "
            <div class=\"card block\">
                <div class=\"card-heading\" id={name}>
                    <code class=\"card-header-title\"><p>{name}</p></code>
                </div>
            </div>
            "
            )
        })
        .collect();
    format!(
        "<div class=\"panel block\">
            <div class=\"panel-heading\">
                <p class=\"subtitle\">
                    Type doc:
                </p>
            </div>
        </div>
        <div class=\"panel block\">
            <div class=\"panel-heading\">
                <p class=\"subtitle\">
                    Variants:
                </p>
            </div>
            <div class=\"panel-block\">
                <div class=\"container\">
                    {variants}
                </div>
            </div>

        </div>"
    )
}

pub fn get_record_type(type_def: &RecordGenerator) -> String {
    let fields = dedupe_by(type_def.fields.iter(), |(name_container, _)| {
        name_container.to_owned()
    })
    .map(|(name, written_type)| {
        let doc = get_doc_record(type_def, name);
        let written_type = parse_namepart(written_type);
        let name = String::from_utf8_lossy(name);
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
                let signature = parse_namepart(&v.signature);

                let meta_method = if v.is_meta_method { "metamethod" } else { "" };
                let method_name = String::from_utf8_lossy(&v.name);
                format!("{meta_method} {method_name}: {signature}")
            };
            let documentation = get_doc_record(type_def, &v.name);
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

pub(crate) fn get_type(type_def: &TypeGenerator) -> String {
    match type_def {
        TypeGenerator::Record(x) => get_record_type(x),
        TypeGenerator::Enum(x) => get_enum_type(x),
    }
}

pub(crate) fn gen_type_page(type_def: &TypeGenerator, side_bar: String) -> String {
    let (page, name) = match type_def {
        TypeGenerator::Record(x) => (
            get_record_type(x),
            tealr::type_parts_to_str(x.type_name.clone()),
        ),
        TypeGenerator::Enum(x) => (get_enum_type(x), tealr::type_parts_to_str(x.name.clone())),
    };

    gen_doc(&page, &name, side_bar)
}

pub(crate) fn gen_sidebar_item(link_to: &Path, field_name: &NameContainer) -> String {
    let link_to = link_to.to_string_lossy();
    let field_name = String::from_utf8_lossy(field_name);
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

pub(crate) fn type_should_be_inlined(v: &TypeGenerator) -> bool {
    match v {
        TypeGenerator::Record(x) => x.should_be_inlined,
        TypeGenerator::Enum(_) => false,
    }
}

pub(crate) fn gen_side_bar(
    all_types: &TypeWalker,
    current: Option<&TypeGenerator>,
    index: &str,
    path: &Path,
) -> String {
    all_types
        .global_instances_off
        .iter()
        .map(|v| {
            (
                v.name.to_string(),
                index.to_string() + "#" + &v.name,
                "".to_string(),
                "".to_string(),
            )
        })
        .chain(
            (all_types
                .given_types
                .iter()
                .filter(|v| type_should_be_inlined(v)))
            .chain(
                all_types
                    .given_types
                    .iter()
                    .filter(|v| !type_should_be_inlined(v)),
            )
            .map(|v| {
                let (name, link_to) = if type_should_be_inlined(v) {
                    (index.to_string(), "index".to_string())
                } else {
                    let x = get_type_name(v).to_string();
                    (x.clone(), x)
                };
                let link_to = path.join(link_to);

                let is_active = current
                    .map(|current| get_type_name(v) == get_type_name(current))
                    .unwrap_or_else(|| type_should_be_inlined(v))
                    .then(|| "is-active")
                    .unwrap_or("");
                let res = match v {
                    TypeGenerator::Record(v) => {
                        let fields = dedupe_by(v.fields.iter(), |(v, _)| v.to_owned())
                            .map(|(field_name, _)| gen_sidebar_item(&link_to, field_name))
                            .collect::<String>();
                        let fields = gen_sidebar_tab(fields, "Fields");
                        let methods = create_methods_iter(v)
                            .map(|(_, v)| v)
                            .map(|v| gen_sidebar_item(&link_to, &v.name))
                            .collect::<String>();
                        let methods = gen_sidebar_tab(methods, "Methods");

                        format!(
                            "
                       {fields}
                       {methods}
                        "
                        )
                    }
                    TypeGenerator::Enum(x) => {
                        let items = x
                            .variants
                            .iter()
                            .map(|v| gen_sidebar_item(&link_to, v))
                            .collect();
                        gen_sidebar_tab(items, "variants")
                    }
                };
                let link_to_as_string = link_to.to_string_lossy().to_string() + ".html";
                (name, link_to_as_string, is_active.to_string(), res)
            }),
        )
        .map(|(name, link_to_as_string, is_active, res)| {
            format! {
                    "<li>
                    <a href=\"{link_to_as_string}\" class=\"{is_active}\">
                        <span class=\"icon\"><i class=\"fa fa-file\"></i></span> {name}
                    </a>
                    <ul>
                    {res}
                    </ul>
                </li>"
            }
        })
        .collect::<String>()
}

pub fn generate_global(global: &tealr::GlobalInstance, base_url: &Path) -> String {
    let name = &global.name;
    let type_name = parse_namepart(&global.teal_type);

    let docs = if !global.doc.is_empty() {
        let x = parse_markdown(&global.doc);
        format!("<div class=\"card-content content\">{x}</div>")
    } else {
        String::from("")
    };

    format!(
        "
        <div class=\"card block\">
            <div class=\"card-header\">
                <code class=\"card-header-title\" id=\"{name}\">
                    {name}:{type_name}
                </code>
            </div>
            {docs}
        </div>
    
    "
    )
}
