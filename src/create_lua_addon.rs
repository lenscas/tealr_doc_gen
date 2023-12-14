use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;
use serde::Serialize;
use std::io::Write;
use tealr::{
    ExportedFunction, FunctionRepresentation, MapRepresentation, RecordGenerator, Type, TypeWalker,
};
use zip::write::FileOptions;

use crate::app::LuaAddon;

#[derive(Serialize)]
struct LuaAddonConfig {
    name: String,
    words: Vec<String>,
    files: Vec<String>,
    settings: HashMap<String, serde_json::Value>,
}

pub fn create_lua_addon(
    addon_config: LuaAddon,
    type_defs: TypeWalker,
    mut path: PathBuf,
    is_global: bool,
    library_name: &str,
) -> anyhow::Result<bool> {
    let config = match addon_config {
        LuaAddon::False => return Ok(false),
        LuaAddon::Create {
            words,
            files,
            settings,
        } => LuaAddonConfig {
            name: library_name.to_owned(),
            words,
            files,
            settings,
        },
    };
    let zip_name = {
        let mut x = String::new();
        x += &config.name;
        x += ".zip";
        x
    };
    path.push(&zip_name);
    let x = std::fs::File::create(&path).context("Could not create zipfile for lua addon")?;
    let mut x = zip::write::ZipWriter::new(x);
    x.start_file("plugin.json", FileOptions::default())?;
    x.write_all(
        serde_json::to_string_pretty(&config)
            .context("Could not create plugin.json")?
            .as_bytes(),
    )
    .context("Error while writing plugin.json")?;
    x.add_directory("library", FileOptions::default())
        .context("Could not create library folder")?;
    x.start_file(
        format!("library/{}.lua", &config.name),
        FileOptions::default(),
    )
    .context("Could not create library.lua file")?;
    let res = to_lua(&config.name, type_defs, is_global);
    x.write(res.as_bytes())
        .context("Failed writing library definition to library.lua")?;
    x.flush()
        .context("Could not write lua language server addon to disk")?;
    drop(x);
    Ok(true)
}

fn to_lua(name: &str, type_defs: TypeWalker, is_global: bool) -> String {
    let mut file = String::with_capacity(type_defs.given_types.len());
    file.push_str("---@meta\n\n");

    let mut base_fields = String::with_capacity(type_defs.given_types.len());
    let mut base_methods = String::with_capacity(type_defs.given_types.len());

    let mut classes = String::with_capacity(type_defs.given_types.len());
    let mut class_fields = String::new();
    let mut class_methods = String::new();

    let mut class_name = String::new();
    for ty in type_defs.given_types {
        match ty {
            tealr::TypeGenerator::Record(x) => {
                if x.should_be_inlined {
                    write_record_as_class(&mut base_methods, &mut base_fields, "", name, &x, false)
                } else {
                    class_fields.clear();
                    class_methods.clear();
                    class_name.clear();
                    //class_name.push_str(name);
                    //class_name.push('.');
                    class_name.push_str(&tealr::type_parts_to_str(x.type_name.clone()));
                    write_record_as_class(
                        &mut class_methods,
                        &mut class_fields,
                        "",
                        &class_name,
                        &x,
                        true,
                    );
                    classes += &class_fields;
                    classes += &class_methods;
                }
            }
            tealr::TypeGenerator::Enum(x) => {
                classes.push_str("---@alias ");
                //classes.push_str(name);
                //classes.push('.');
                classes.push_str(&tealr::type_parts_to_str(x.name));
                classes.push('\n');
                for variant in x.variants {
                    classes.push_str("---|'\"");
                    classes.push_str(&String::from_utf8_lossy(&variant));
                    classes.push_str("\"'\n");
                }
            }
        }
    }
    let mut write_function_to = String::new();
    for global in type_defs.global_instances_off {
        if let Type::Function(x) = &global.ty {
            write_function_to.clear();
            write_function(
                #[allow(deprecated)]
                &ExportedFunction {
                    name: global.name.clone().into(),
                    signature: Vec::new().into(),
                    params: x.params.to_owned(),
                    returns: x.returns.to_owned(),
                    is_meta_method: false,
                },
                "",
                "",
                &mut write_function_to,
                &mut classes,
            )
        } else {
            classes.push_str("---@type ");
            classes.push_str(&type_to_name(&global.ty, ""));
            classes.push('\n');
            classes.push_str(&global.name);
            classes.push_str(" = nil");
            classes.push('\n');
        }
    }
    file.push_str(&base_fields);
    if !is_global {
        file.push_str("local ");
    }
    file.push_str(name);
    file.push_str(" = {}\n");
    file.push_str(&base_methods);
    file.push_str(&classes);
    file.push_str("\n return ");
    file.push_str(name);
    file
}

fn write_record_as_class(
    methods: &mut String,
    fields: &mut String,
    base: &str,
    class_name: &str,
    generator: &RecordGenerator,
    write_class: bool,
) {
    fields.reserve(generator.fields.len());
    methods.reserve(generator.functions.len() + generator.methods.len());
    if write_class {
        fields.push_str("---@class ");
        fields.push_str(class_name);
        fields.push('\n');
    }
    for field in &generator.fields {
        fields.push_str("---@field ");
        fields.push_str(&String::from_utf8_lossy(&field.name));
        fields.push(' ');
        fields.push_str(&type_to_name(&field.ty, base));
        if let Some(x) = generator.documentation.get(&field.name) {
            fields.push(' ');
            fields.push_str(&x.replace('\n', "<br>"));
        }
        fields.push('\n');
    }
    if write_class {
        methods.push_str("local ");
        methods.push_str(&tealr::type_parts_to_str(generator.type_name.clone()));
        methods.push_str(" = {}\n");
    }
    if !generator.functions.is_empty() {
        let mut function_written_out = String::new();
        generator
            .functions
            .iter()
            .chain(generator.mut_functions.iter())
            .chain(generator.methods.iter())
            .chain(generator.mut_methods.iter())
            .chain(generator.meta_function.iter())
            .chain(generator.meta_function_mut.iter())
            .chain(generator.meta_method.iter())
            .chain(generator.meta_method_mut.iter())
            .for_each(|function| {
                write_function(
                    function,
                    class_name,
                    base,
                    &mut function_written_out,
                    methods,
                )
            });
    }
}

fn write_function(
    function: &ExportedFunction,
    class_name: &str,
    base: &str,
    function_written_out: &mut String,
    methods: &mut String,
) {
    let generics = function.get_generics();

    for generic in generics {
        methods.push_str("---@generic ");
        methods.push_str(&generic.to_string());
        methods.push('\n');
    }
    function_written_out.clear();
    function_written_out.push_str("function ");
    if !class_name.is_empty() {
        function_written_out.push_str(class_name);
        function_written_out.push('.');
    }
    function_written_out.push_str(&String::from_utf8_lossy(&function.name));
    function_written_out.push('(');
    if !function.params.is_empty() {
        for (key, param) in function.params.iter().enumerate() {
            methods.push_str("---@param ");
            let param_name = param
                .param_name
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("Param{}", key + 1));
            methods.push_str(&param_name);
            function_written_out.push_str(&param_name);
            function_written_out.push(',');
            methods.push(' ');
            methods.push_str(&type_to_name(&param.ty, base));

            methods.push('\n')
        }
        function_written_out.pop();
    }
    function_written_out.push_str(") end\n");
    for returned in &function.returns {
        methods.push_str("---@return ");
        methods.push_str(&type_to_name(returned, base));
        methods.push('\n');
    }
    methods.push_str(function_written_out);
}

fn type_to_name(ty: &Type, base: &str) -> String {
    match ty {
        Type::Function(FunctionRepresentation { params, returns }) => {
            let mut name = "fun(".to_string();
            if !params.is_empty() {
                for (key, param) in params.iter().enumerate() {
                    let param_name = param
                        .param_name
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| format!("Param{}", key + 1));
                    name += &param_name;
                    name += ":";
                    name += &type_to_name(&param.ty, base);
                    name += ",";
                }
                name.pop();
            }
            name += "):";
            if !returns.is_empty() {
                for returned in returns {
                    name += &type_to_name(returned, base);
                    name += ",";
                }
                name.pop();
            } else {
                name += "nil"
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
            let x = type_to_name(x, base);
            format! {"({})[]",x}
        }
        Type::Map(MapRepresentation { key, value }) => {
            let key = type_to_name(key, base);
            let value = type_to_name(value, base);
            format!("{{ [{}]: {}}}", key, value)
        }
        Type::Or(x) => {
            let mut name = String::with_capacity(x.len() * 2 + 4);
            name.push_str("( ");
            for part in x.iter().map(|v| type_to_name(v, base)) {
                name += &part;
                name.push('|');
            }
            name.pop();
            name.push_str(" )");
            name
        }
        Type::Tuple(x) => {
            let mut name = String::with_capacity(x.len() * 2);
            for part in x.iter().map(|v| type_to_name(v, base)) {
                name += &part;
                name.push(',');
            }
            name.pop();
            name
        }
    }
}
