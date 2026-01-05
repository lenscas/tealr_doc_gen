use tealr::{
    Field, FunctionRepresentation, NameContainer, RecordGenerator, SingleType, Type, TypeGenerator,
    TypeWalker,
};

#[derive(Clone, tealr::mlu::FromToLua, tealr::ToTypename)]
pub struct NameAndSignature {
    name: NameContainer,
    signature: Type,
}

#[derive(Clone, tealr::mlu::FromToLua, tealr::ToTypename)]
///Defines how another type uses this type
pub struct User {
    ///the type that references the current type
    ty: SingleType,
    ///the name of the methods that use this type as a parameter
    as_params: Vec<NameAndSignature>,
    ///the name of the methods that have this type as a return type
    as_return: Vec<NameAndSignature>,
    ///the name of the fields that contain this type _somewhere_
    ///
    ///This can be as it contains just this type, or because it is part of a more complex type like an array or a function
    as_fields: Vec<NameAndSignature>,
}

enum FoundAs {
    FunctionArgument,
    FunctionReturn,
    Field,
}

fn drill_to_find(ty: &Type, to_find: &SingleType) -> Option<FoundAs> {
    match ty {
        Type::Function(function_representation) => function_representation
            .params
            .iter()
            .map(|v| drill_to_find(&v.ty, to_find).map(|_| FoundAs::FunctionArgument))
            .find(|x| x.is_some())
            .or_else(|| {
                function_representation
                    .returns
                    .iter()
                    .map(|v| drill_to_find(v, to_find).map(|_| FoundAs::FunctionReturn))
                    .find(|x| x.is_some())
            })
            .flatten(),
        Type::Single(single_type) => {
            if single_type.name == to_find.name {
                Some(FoundAs::Field)
            } else {
                None
            }
        }
        Type::Map(map_representation) => drill_to_find(&map_representation.key, to_find)
            .or_else(|| drill_to_find(&map_representation.value, to_find)),
        Type::Or(items) | Type::Tuple(items) => items
            .iter()
            .map(|x| drill_to_find(x, to_find))
            .next()
            .flatten(),
        Type::Array(x) | Type::Variadic(x) => drill_to_find(x, to_find),
    }
}

pub fn find_users(teal_type: &tealr::TypeGenerator, type_walker: &TypeWalker) -> Vec<User> {
    let Some(name_to_find) = teal_type.type_name().single() else {
        return Default::default();
    };
    let inlined_type = type_walker
        .given_types
        .iter()
        .find(|x| x.is_inlined())
        .map(|v| v.type_name().to_owned())
        .unwrap_or_else(|| Type::new_single("index", tealr::KindOfType::External));

    let globals = RecordGenerator {
        should_be_inlined: true,
        is_user_data: false,
        ty: inlined_type,
        fields: type_walker
            .global_instances_off
            .iter()
            .map(|v| Field {
                name: v.to_owned().name.into(),
                ty: v.ty.to_owned(),
            })
            .collect(),
        static_fields: Default::default(),
        methods: Default::default(),
        mut_methods: Default::default(),
        functions: Default::default(),
        mut_functions: Default::default(),
        meta_method: Default::default(),
        meta_method_mut: Default::default(),
        meta_function: Default::default(),
        meta_function_mut: Default::default(),
        documentation: Default::default(),
        type_doc: Default::default(),
        next_docs: Default::default(),
        should_generate_help_method: Default::default(),
    };
    type_walker
        .iter()
        .filter_map(TypeGenerator::record)
        .chain(Some(&globals))
        .filter_map(|v| {
            let Type::Single(single_type) = &v.ty else {
                return None;
            };
            if single_type.name == name_to_find.name {
                return None;
            }
            let mut params = Vec::new();
            let mut returns = Vec::new();
            let mut fields = Vec::new();
            v.fields
                .iter()
                .map(|field| (&field.name, &field.ty))
                .filter_map(|(name, ty)| {
                    drill_to_find(ty, name_to_find).map(|v| {
                        (
                            v,
                            NameAndSignature {
                                name: name.to_owned(),
                                signature: ty.clone(),
                            },
                        )
                    })
                })
                .chain(v.all_functions().filter_map(|y| {
                    y.params
                        .iter()
                        .filter_map(|x| {
                            drill_to_find(&x.ty, name_to_find).map(|_| {
                                (
                                    FoundAs::FunctionArgument,
                                    NameAndSignature {
                                        name: y.name.clone(),
                                        signature: Type::Function(FunctionRepresentation {
                                            params: y.params.clone(),
                                            returns: y.returns.clone(),
                                        }),
                                    },
                                )
                            })
                        })
                        .next()
                        .or_else(|| {
                            y.returns
                                .iter()
                                .filter_map(|x| {
                                    drill_to_find(x, name_to_find).map(|_| {
                                        (
                                            FoundAs::FunctionReturn,
                                            NameAndSignature {
                                                name: y.name.clone(),
                                                signature: Type::Function(FunctionRepresentation {
                                                    params: y.params.clone(),
                                                    returns: y.returns.clone(),
                                                }),
                                            },
                                        )
                                    })
                                })
                                .next()
                        })
                }))
                .for_each(|(found_as, member)| match found_as {
                    FoundAs::FunctionArgument => params.push(member),
                    FoundAs::FunctionReturn => returns.push(member),
                    FoundAs::Field => {
                        fields.push(member);
                    }
                });
            if params.is_empty() && returns.is_empty() && fields.is_empty() {
                return None;
            }
            Some(User {
                ty: single_type.to_owned(),
                as_params: params,
                as_return: returns,
                as_fields: fields,
            })
        })
        .collect()
}
