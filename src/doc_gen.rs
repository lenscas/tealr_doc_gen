use std::borrow::Cow;

use tealr::TypeGenerator;

pub(crate) fn get_type_name(type_def: &TypeGenerator) -> Cow<'static, str> {
    let z = match type_def {
        TypeGenerator::Record(x) => &x.type_name,
        TypeGenerator::Enum(x) => &x.name,
    };
    tealr::type_parts_to_str(z.clone())
}

pub(crate) fn type_should_be_inlined(v: &TypeGenerator) -> bool {
    match v {
        TypeGenerator::Record(x) => x.should_be_inlined,
        TypeGenerator::Enum(_) => false,
    }
}
