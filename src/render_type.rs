use std::path::PathBuf;

use tealr::{
    mlu::{
        mlua::{self, FromLua, IntoLua, Table, UserDataRef},
        TypedFunction,
    },
    ExportedFunction, FunctionRepresentation, KindOfType, Name, RecordGenerator, SingleType,
    ToTypename, Type, TypeBody, TypeGenerator,
};

pub struct RenderOptions<X: ToTypename> {
    function: TypedFunction<(FunctionRepresentation, X), String>,
    single: TypedFunction<(tealr::SingleType, X), String>,
    map: TypedFunction<(tealr::MapRepresentation, X), String>,
    or: TypedFunction<(Vec<tealr::Type>, X), String>,
    array: TypedFunction<(tealr::Type, X), String>,
    tuple: TypedFunction<(Vec<tealr::Type>, X), String>,
    variadic: TypedFunction<(tealr::Type, X), String>,
}

impl<X: ToTypename> tealr::TypeBody for RenderOptions<X> {
    fn get_type_body() -> TypeGenerator {
        let mut record_generator = RecordGenerator::new::<RenderOptions<X>>(false);
        record_generator.document_type(
            "Used to tell [type_to_string](index#type_to_string) how to render each variant of a type",
        );
        record_generator
            .add_field::<_, TypedFunction<(FunctionRepresentation, X), String>>("[\"function\"]");
        record_generator.add_field::<_, TypedFunction<(tealr::SingleType, X), String>>("single");
        record_generator
            .add_field::<_, TypedFunction<(tealr::MapRepresentation, X), String>>("map");
        record_generator.add_field::<_, TypedFunction<(Vec<tealr::Type>, X), String>>("[\"or\"]");
        record_generator.add_field::<_, TypedFunction<(tealr::Type, X), String>>("array");
        record_generator.add_field::<_, TypedFunction<(Vec<tealr::Type>, X), String>>("tuple");
        record_generator.add_field::<_, TypedFunction<(tealr::Type, X), String>>("variadic");
        TypeGenerator::Record(Box::new(record_generator))
    }
}

impl<X: ToTypename + IntoLua> IntoLua for RenderOptions<X> {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let table = lua.create_table()?;
        table.set("function", self.function)?;
        table.set("single", self.single)?;
        table.set("map", self.map)?;
        table.set("or", self.or)?;
        table.set("array", self.array)?;
        table.set("tuple", self.tuple)?;
        table.set("variadic", self.variadic)?;
        Ok(mlua::Value::Table(table))
    }
}
impl<X: ToTypename> ToTypename for RenderOptions<X> {
    fn to_typename() -> tealr::Type {
        tealr::Type::new_single_with_generics(
            "RenderOptions",
            tealr::KindOfType::External,
            vec![X::to_typename()],
        )
    }
}

impl<X: ToTypename + FromLua> FromLua for RenderOptions<X> {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let table = mlua::Table::from_lua(value, lua)?;
        Ok(RenderOptions {
            function: table.get("function")?,
            single: table.get("single")?,
            map: table.get("map")?,
            or: table.get("or")?,
            array: table.get("array")?,
            tuple: table.get("tuple")?,
            variadic: table.get("variadic")?,
        })
    }
}
pub fn render_type<X: ToTypename + IntoLua>(
    ty: &tealr::Type,
    options: RenderOptions<X>,
    extra_value: X,
) -> mlua::Result<String> {
    let res = match ty.clone() {
        tealr::Type::Function(function_representation) => options
            .function
            .call((function_representation, extra_value)),
        tealr::Type::Single(single_type) => options.single.call((single_type, extra_value)),
        tealr::Type::Map(map_representation) => options.map.call((map_representation, extra_value)),
        tealr::Type::Or(items) => options.or.call((items, extra_value)),
        tealr::Type::Array(items) => options.array.call((*items, extra_value)),
        tealr::Type::Tuple(items) => options.tuple.call((items, extra_value)),
        tealr::Type::Variadic(ty) => options.variadic.call((*ty, extra_value)),
    }?;
    Ok(res)
}

pub fn find_all_generics(ty: &tealr::Type) -> Vec<Type> {
    let mut generics = Vec::new();
    match ty {
        tealr::Type::Single(single_type) => {
            if single_type.kind.is_generic() {
                generics.push(Type::Single(single_type.clone()));
            }
            single_type
                .generics
                .iter()
                .map(find_all_generics)
                .for_each(|x| {
                    generics.extend(x);
                });
        }
        tealr::Type::Map(map_representation) => {
            generics.extend(find_all_generics(&map_representation.key));
            generics.extend(find_all_generics(&map_representation.value));
        }
        tealr::Type::Tuple(items) | tealr::Type::Or(items) => {
            generics.extend(items.iter().flat_map(find_all_generics));
        }
        tealr::Type::Variadic(x) | tealr::Type::Array(x) => {
            generics.extend(find_all_generics(x));
        }
        tealr::Type::Function(function_representation) => {
            let x = ExportedFunction {
                name: std::borrow::Cow::from("foo").into(),
                params: function_representation.params.to_owned(),
                returns: function_representation.returns.to_owned(),
                is_meta_method: false,
            };
            generics.extend(x.get_generic_types().into_iter().map(|v| v.to_owned()));
        }
    }
    generics
}

pub fn is_type_compatible(ty: &tealr::SingleType, other: tealr::SingleType) -> bool {
    let other = SingleTypeNoConsume::from(other);
    is_type_compatible_helper(ty, &other)
}

fn is_type_compatible_helper(ty: &tealr::SingleType, other: &SingleTypeNoConsume) -> bool {
    ty.name == other.name && ty.generics.len() == other.generics.len()
}
enum OwnedOrNot {
    Owned(Vec<Type>),
    Reference(Vec<UserDataRef<Type>>),
}
impl OwnedOrNot {
    fn len(&self) -> usize {
        match self {
            OwnedOrNot::Owned(user_data_refs) => user_data_refs.len(),
            OwnedOrNot::Reference(user_data_refs) => user_data_refs.len(),
        }
    }
}

pub(crate) struct SingleTypeNoConsume {
    name: Name,
    kind: KindOfType,
    generics: OwnedOrNot,
}

impl FromLua for SingleTypeNoConsume {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let table = Table::from_lua(value, lua)?;
        let generics = Vec::<UserDataRef<Type>>::from_lua(table.get("generics")?, lua)?;
        let name = Name::from_lua(table.get("name")?, lua)?;
        let kind = KindOfType::from_lua(table.get("kind")?, lua)?;
        Ok(Self {
            generics: OwnedOrNot::Reference(generics),
            name,
            kind,
        })
    }
}

impl IntoLua for SingleTypeNoConsume {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let table = lua.create_table()?;
        let generics = match self.generics {
            OwnedOrNot::Owned(user_data_refs) => user_data_refs
                .into_iter()
                .map(|x| x.clone().into_lua(lua))
                .collect::<Result<Vec<_>, _>>()?,
            OwnedOrNot::Reference(user_data_refs) => user_data_refs
                .into_iter()
                .map(|x| x.clone().into_lua(lua))
                .collect::<Result<Vec<_>, _>>()?,
        };
        table.set("generics", generics)?;
        table.set("name", self.name)?;
        table.set("kind", self.kind)?;
        table.into_lua(lua)
    }
}
impl TypeBody for SingleTypeNoConsume {
    fn get_type_body() -> TypeGenerator {
        SingleType::get_type_body()
    }
}

impl From<SingleType> for SingleTypeNoConsume {
    fn from(value: SingleType) -> Self {
        Self {
            generics: OwnedOrNot::Owned(value.generics),
            kind: value.kind,
            name: value.name,
        }
    }
}

impl ToTypename for SingleTypeNoConsume {
    fn to_typename() -> Type {
        SingleType::to_typename()
    }

    fn to_function_param() -> Vec<tealr::FunctionParam> {
        SingleType::to_function_param()
    }
}

pub fn type_to_link_url(
    ty: impl Into<SingleTypeNoConsume>,
    all_types: &[TypeGenerator],
    mut link_path: PathBuf,
) -> Option<PathBuf> {
    let ty = ty.into();
    let mut x = all_types
        .iter()
        .filter_map(|v| match v.type_name() {
            tealr::Type::Single(x) => Some((x, v)),
            _ => None,
        })
        .filter(|(v, _)| is_type_compatible_helper(v, &ty))
        .take(2);
    let Some((found_type, generator)) = x.next() else {
        if ty.name == Name::from("index") {
            return Some(link_path);
        }
        eprintln!("Tried making link to unknown type: {}", ty.name);
        return None;
    };
    if x.next().is_some() {
        eprintln!("Multiple types with the same name: {}", found_type.name);
    }
    let name = if generator.is_inlined() {
        "index".to_string()
    } else {
        found_type.name.to_string()
    };
    link_path.push(name);
    link_path.set_extension("html");
    Some(link_path)
}
