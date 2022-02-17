use tealr::mlu::mlua::Table;

pub(crate) fn compile_teal(code: String) -> String {
    let lua = tealr::mlu::mlua::Lua::new();
    let code = code
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("'", "\\'")
        .replace("\n", "\\n");

    let code = format!(
        "
local tl = require('tl')
local env = tl.init_env(false, false, true)
local output, result = tl.gen('{code}', env)
return {{ output, result.syntax_errors, result.type_errors }}
"
    );
    let table: Table = lua.load(&code).eval().unwrap();
    println!(
        "type_errors: {:?}",
        table.get::<_, Vec<tealr::mlu::mlua::Value>>(3).unwrap()
    );
    println!(
        "syntax_errors: {:?}",
        table.get::<_, Vec<tealr::mlu::mlua::Value>>(2).unwrap()
    );
    table.get(1).unwrap()
}
