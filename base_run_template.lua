local etlua = etlua()
local template, err = etlua.compile(template)
if not template then print(err) end
return template()
