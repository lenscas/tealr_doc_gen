use std::collections::HashSet;
use tealr::{FunctionParam, FunctionRepresentation, MapRepresentation, Type, TypeGenerator};

pub(crate) fn get_type_name(type_def: &TypeGenerator) -> String {
    let ty = type_def.type_name();
    type_to_name(
        ty,
        "",
        TypeConfig {
            part_off: TypeIsPartOf::None,
            known_generics: Default::default(),
        },
    )
}

pub(crate) fn type_should_be_inlined(v: &TypeGenerator) -> bool {
    match v {
        TypeGenerator::Record(x) => x.should_be_inlined,
        TypeGenerator::Enum(_) => false,
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum TypeIsPartOf {
    None,
    FunctionParameter,
    FunctionReturn,
}

#[derive(Clone)]
pub struct TypeConfig {
    pub part_off: TypeIsPartOf,
    pub known_generics: HashSet<Type>,
}

fn get_param_name(param: &FunctionParam) -> Option<String> {
    if let Type::Variadic(_) = param.ty {
        Some("...".into())
    } else {
        param.param_name.as_ref().map(ToString::to_string)
    }
}
fn get_missing_generics<'a>(
    types: impl IntoIterator<Item = &'a Type>,
    generics: &'a HashSet<Type>,
) -> impl Iterator<Item = &'a Type> {
    let mut found_generics = HashSet::new();
    types
        .into_iter()
        .filter(|x| !generics.contains(x))
        .filter(move |v| {
            if found_generics.contains(v) {
                false
            } else {
                found_generics.insert(*v);
                true
            }
        })
}

fn type_to_name(ty: &Type, base: &str, config: TypeConfig) -> String {
    match ty {
        Type::Function(FunctionRepresentation { params, returns }) => {
            let mut name = "function".to_string();
            let generics = get_missing_generics(
                params.iter().map(|x| &x.ty).chain(returns.iter()),
                &config.known_generics,
            )
            .cloned()
            .collect::<Vec<_>>();
            let config = TypeConfig {
                part_off: config.part_off,
                known_generics: generics
                    .clone()
                    .into_iter()
                    .chain(config.known_generics)
                    .collect(),
            };
            if !generics.is_empty() {
                name += "<";
                for generic in generics {
                    name += &type_to_name(&generic, base, config.clone());
                    name += ",";
                }
                name.pop();
                name += ">"
            }
            name += &params
                .iter()
                .map(|v| {
                    let mut param_name = get_param_name(v)
                        .map(|mut v| {
                            v.push_str(" : ");
                            v
                        })
                        .unwrap_or_default();
                    param_name += &type_to_name(
                        &v.ty,
                        base,
                        TypeConfig {
                            part_off: TypeIsPartOf::FunctionParameter,
                            known_generics: config.known_generics.clone(),
                        },
                    );
                    param_name
                })
                .collect::<Vec<String>>()
                .join(" , ");
            name += ")";
            if !returns.is_empty() {
                name += ":";
                let returns_as_str = returns
                    .iter()
                    .map(|x| {
                        type_to_name(
                            x,
                            base,
                            TypeConfig {
                                part_off: TypeIsPartOf::FunctionReturn,
                                known_generics: config.known_generics.clone(),
                            },
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(",");
                if returns.len() == 1 || config.part_off == TypeIsPartOf::None {
                    name += &returns_as_str;
                } else {
                    name += "(";
                    name += &returns_as_str;
                    name += ")";
                }
            }
            name
        }
        Type::Single(x) => {
            let name = x.name.to_string();
            let full_name = if x.kind.is_external() {
                let mut string = String::new();
                if !base.is_empty() {
                    string.push_str(base);
                    string.push('.');
                }
                string.push_str(&name);
                string
            } else {
                name
            };
            //a multiple return type tends to be written as `T...`
            //however, this is not how lua-language-server expects it. Then it should be `T ...`
            //so, we check for it following this format and then fixing it to be compatible with lua-language-server
            full_name
                .strip_suffix("...")
                .map(|v| v.trim_end().to_string() + " ...")
                .unwrap_or(full_name)
        }
        Type::Array(x) => {
            let x = type_to_name(
                x,
                base,
                TypeConfig {
                    part_off: TypeIsPartOf::None,
                    known_generics: config.known_generics,
                },
            );
            format! {"{{{}}}",x}
        }
        Type::Map(MapRepresentation { key, value }) => {
            let key = type_to_name(
                key,
                base,
                TypeConfig {
                    part_off: TypeIsPartOf::None,
                    known_generics: config.known_generics.clone(),
                },
            );
            let value = type_to_name(
                value,
                base,
                TypeConfig {
                    part_off: TypeIsPartOf::None,
                    known_generics: config.known_generics,
                },
            );
            format!("{{ {}: {}}}", key, value)
        }
        Type::Or(x) => {
            let mut name = String::with_capacity(x.len() * 2 + 4);
            name.push_str("( ");
            name += &x
                .iter()
                .map(move |v| {
                    type_to_name(
                        v,
                        base,
                        TypeConfig {
                            part_off: TypeIsPartOf::None,
                            known_generics: config.known_generics.clone(),
                        },
                    )
                })
                .collect::<Vec<String>>()
                .join(" | ");
            name.push_str(" )");
            name
        }
        Type::Tuple(x) => {
            let mut name = String::with_capacity(x.len() * 2);
            for part in x.iter().map(move |v| {
                type_to_name(
                    v,
                    base,
                    TypeConfig {
                        part_off: TypeIsPartOf::None,
                        known_generics: config.known_generics.clone(),
                    },
                )
            }) {
                name += &part;
                name.push(',');
            }
            name.pop();
            name
        }
        Type::Variadic(x) => {
            let mut name = type_to_name(
                x,
                base,
                TypeConfig {
                    part_off: TypeIsPartOf::None,
                    known_generics: config.known_generics,
                },
            );
            if config.part_off == TypeIsPartOf::FunctionReturn {
                name += " ...";
            }
            name
        }
    }
}
