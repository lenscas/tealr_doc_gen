use tealr::mlu::mlua::Table;

#[derive(Debug)]
pub(crate) struct CompileResult {
    pub(crate) compiled: Option<String>,
    pub(crate) syntax_errors: Vec<String>,
    pub(crate) type_errors: Vec<String>,
}

pub(crate) fn compile_teal(code: &str) -> CompileResult {
    let lua = tealr::mlu::mlua::Lua::new();
    let code = code
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
        .replace('\n', "\\n");

    let code = format!(
        "
local tl = require('tl')
local env = tl.init_env(false, false, true)
local output, result = tl.gen('{code}', env)
local type_errors = {{}}
for k,v in ipairs(result.type_errors) do
    type_errors[#type_errors + 1] = v.msg .. ' x=' .. tostring(v.x) .. ' y=' .. tostring(v.y) .. '\\n'
end
local syntax_errors = {{}}
for k,v in ipairs(result.syntax_errors) do
    syntax_errors[#syntax_errors + 1] = v.msg .. ' x=' .. tostring(v.x) .. ' y=' .. tostring(v.y) .. '\\n'
end

return {{ output, syntax_errors, type_errors }}
"
    );
    let table: Table = lua.load(&code).eval().unwrap();

    //println!("{}", code);
    CompileResult {
        compiled: table.get::<i64, Option<String>>(1).unwrap_or_default(),
        syntax_errors: table
            .get::<i64, Table>(2)
            .map(|v| v.sequence_values::<String>().map(|v| v.unwrap()).collect())
            .unwrap(),
        type_errors: table
            .get::<i64, Table>(3)
            .map(|v| v.sequence_values::<String>().map(|v| v.unwrap()).collect())
            .unwrap(),
    }
}
