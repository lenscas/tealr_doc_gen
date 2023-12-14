use tealr::{NameContainer, TypeWalker};

#[derive(Clone, tealr::mlu::FromToLua, tealr::ToTypename)]
///Defines how another type uses this type
pub struct User {
    ///the name of the type that uses this type
    type_name: String,
    ///the name of the methods that use this type as a parameter
    as_params: Vec<NameContainer>,
    ///the name of the methods that have this type as a return type
    as_return: Vec<NameContainer>,
    ///the name of the fields that contain this type _somewhere_
    ///
    ///This can be as it contains just this type, or because it is part of a more complex type like an array or a function
    as_fields: Vec<NameContainer>,
}

pub fn find_users(teal_type: &tealr::TypeGenerator, type_walker: &TypeWalker) -> Vec<User> {
    let name_to_find = tealr::type_parts_to_str(match teal_type {
        tealr::TypeGenerator::Record(x) => x.type_name.clone(),
        tealr::TypeGenerator::Enum(x) => x.name.clone(),
    });

    type_walker
        .iter()
        .filter_map(|x| match x {
            tealr::TypeGenerator::Record(x) => Some(x),
            tealr::TypeGenerator::Enum(_) => None,
        })
        .filter_map(|x| {
            let (returns, params): (Vec<Vec<_>>, Vec<Vec<_>>) = x
                .functions
                .iter()
                .chain(&x.mut_functions)
                .chain(&x.methods)
                .chain(&x.mut_methods)
                .chain(&x.meta_function)
                .chain(&x.meta_function_mut)
                .chain(&x.meta_method)
                .chain(&x.meta_method_mut)
                .map(|x| {
                    let mut found_params = false;
                    let mut found_return = false;
                    let mut parenthesis_amount = 0;
                    let name = x.name.clone();
                    let mut returns = Vec::new();
                    let mut params = Vec::new();
                    #[allow(deprecated)]
                    for v in x.signature.iter() {
                        match v {
                            tealr::NamePart::Symbol(x) => {
                                for y in x.chars() {
                                    if y == '(' {
                                        parenthesis_amount += 1;
                                        if parenthesis_amount == 1 {
                                            if !found_params {
                                                found_params = true;
                                            } else if !found_return {
                                                found_return = true;
                                            }
                                        }
                                    } else if y == ')' {
                                        parenthesis_amount -= 1;
                                    }
                                }
                            }
                            tealr::NamePart::Type(x) => {
                                if x.name == name_to_find {
                                    if found_return {
                                        returns.push(name.clone())
                                    } else if found_params {
                                        params.push(name.clone())
                                    }
                                }
                            }
                        }
                    }
                    (returns, params)
                })
                .unzip();
            let as_fields: Vec<_> = x
                .fields
                .iter()
                .chain(&x.static_fields)
                .filter(|x| {
                    x.teal_type.iter().any(|x| match x {
                        tealr::NamePart::Symbol(_) => false,
                        tealr::NamePart::Type(x) => x.name == name_to_find,
                    })
                })
                .map(|x| x.name.clone())
                .collect();
            let as_params: Vec<_> = params.into_iter().flatten().collect();
            let as_return: Vec<_> = returns.into_iter().flatten().collect();
            if as_params.is_empty() && as_return.is_empty() && as_fields.is_empty() {
                return None;
            }
            Some(User {
                type_name: tealr::type_parts_to_str(x.type_name.clone()).to_string(),
                as_params,
                as_return,
                as_fields,
            })
        })
        .collect()
}
