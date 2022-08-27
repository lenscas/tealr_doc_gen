local etlua = etlua()

function inspect_type(name, tbl)
    if type(tbl) == "table" then
        for k, v in pairs(tbl) do
            local typeOf = type(v)
            if typeOf == "table" then
                print(name, " has ", k, "of", typeOf, "with")
                inspect_type(name .. " - " .. k, v)
            else
                print(name, " has ", k, "of", typeOf, "containing", v)
            end
        end
    else
        print(name, " is ", k, "of", typeOf, "containing", tbl)
    end
end

local template, err = etlua.compile(template)
if not template then print(err) end
return template()
