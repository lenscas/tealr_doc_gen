use tealr::{
    mlu::{self, mlua::UserDataRef, TypedFunction},
    TypeWalker,
};

use crate::render_type::{find_all_generics, render_type, RenderOptions};

mod definition_file;
mod generate_warnings;
mod html;
mod lua_addon;
mod run;
mod sidebar;
pub(crate) use definition_file::generate_self_def;
pub(crate) use html::generate_self_doc;
pub(crate) use run::{run_from_walker, run_template};

fn shared_types(walker: TypeWalker) -> TypeWalker {
    walker
        .process_type::<RenderOptions<mlu::generics::X>>()
        .process_type::<tealr::FunctionParam>()
        .process_type::<tealr::FunctionRepresentation>()
        .process_type::<tealr::MapRepresentation>()
        .process_type::<tealr::SingleType>()
        .process_type::<tealr::Name>()
        .process_type::<tealr::ExtraPage>()
        .process_type::<tealr::Type>()
        .process_type::<tealr::EnumGenerator>()
        .process_type::<tealr::ExportedFunction>()
        .process_type::<tealr::Field>()
        .process_type::<tealr::GlobalInstance>()
        .process_type::<tealr::KindOfType>()
        .process_type::<tealr::NamePart>()
        .process_type::<tealr::RecordGenerator>()
        .process_type::<tealr::TealType>()
        .process_type::<tealr::TypeGenerator>()
        .process_type::<tealr::TypeWalker>()
}

fn shared_globals<T: mlu::InstanceCollector>(
    instance_collector: &mut T,
    etlua: &str,
    template: &str,
) -> mlu::mlua::Result<()> {
    instance_collector.document_instance("function to load etlua");
    instance_collector.add_instance("etlua", |lua| {
        lua.load(etlua).set_name("etlua").into_function()
    })?;
    instance_collector.document_instance("template that gets loaded");
    instance_collector.add_instance("template", |_| Ok(template))?;

    instance_collector.add_instance("find_generics", |lua| {
        TypedFunction::from_rust(
            |_, ty: UserDataRef<tealr::Type>| Ok(find_all_generics(&ty)),
            lua,
        )
    })?;
    instance_collector.document_instance("Removes all duplicate instances of a table using a function to select what to look for. Returns a new table with the duplicates removed");
    instance_collector.document_instance("```teal_lua");
    instance_collector.document_instance("local with_dupes = {1,2,3,3,2,1}");
    instance_collector.document_instance(
        "local without_duplicates = dedupe_by(with_dupes,function(x:integer):integer return x end)",
    );
    instance_collector.document_instance("print(#without_duplicates, #with_dupes)");
    instance_collector.document_instance("```");
    instance_collector.add_instance("dedupe_by", |lua| {
        TypedFunction::from_rust(
            |lua,
             (a, b): (
                Vec<mlu::generics::X>,
                TypedFunction<mlu::generics::X, mlu::generics::Y>,
            )| {
                let table = lua.create_table()?;
                a.into_iter()
                    .filter_map(|x| match b.call(x.clone()) {
                        Ok(to_store) => match table.contains_key(to_store.clone()) {
                            Ok(true) => None,
                            Ok(false) => match table.set(to_store, true) {
                                Ok(()) => Some(Ok(x)),
                                Err(x) => Some(Err(x)),
                            },
                            Err(x) => Some(Err(x)),
                        },
                        Err(x) => Some(Err(x)),
                    })
                    .collect::<Result<Vec<_>, _>>()
            },
            lua,
        )
    })?;
    instance_collector.add_instance("type_to_string", |lua| {
        TypedFunction::from_rust(
            |_,
             (ty, options, extra_value): (
                UserDataRef<tealr::Type>,
                RenderOptions<mlu::generics::X>,
                mlu::generics::X,
            )| render_type(&ty, options, extra_value),
            lua,
        )
    })?;
    Ok(())
}
