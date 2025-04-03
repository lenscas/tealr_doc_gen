use std::path::Path;

use tealr::{mlu::FromToLua, NameContainer, ToTypename, TypeWalker};

use crate::{
    doc_gen::{get_type_name, type_should_be_inlined},
    render_type::type_to_link_url,
    Paths,
};

#[derive(FromToLua, ToTypename, Clone)]
/// An element in the sidebar
pub(super) struct SideBar {
    /// What url to link to
    pub(super) link_to: String,
    /// Name of the type in the sidebar
    pub(super) name: String,
    /// The members of the type
    pub(super) members: Vec<Members>,
}
#[derive(FromToLua, ToTypename, Clone)]
/// A member as shown in the sidebar
pub(super) struct Members {
    /// The name of the member
    pub(super) name: NameContainer,
}

pub fn generate_sidebar_data(
    type_defs: &TypeWalker,
    paths: &Paths,
    link_path: &Path,
) -> Vec<SideBar> {
    type_defs
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
                (paths.name.to_string(), link_path.join("index.html"))
            } else {
                let x = get_type_name(v).to_string();
                let url = match v.type_name() {
                    tealr::Type::Single(single_type) => type_to_link_url(
                        single_type.to_owned(),
                        &type_defs.given_types,
                        link_path.to_path_buf(),
                    ),
                    _ => None,
                }
                .unwrap_or_else(|| {
                    link_path
                        .join(sanitize_filename::sanitize(&x))
                        .with_extension("html")
                });
                (x, url)
            };
            SideBar {
                link_to: link_path.join(link_to).to_string_lossy().into_owned(),
                name,
                members,
            }
        })
        .collect()
}
