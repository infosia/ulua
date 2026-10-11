local function make() return function() return 1 end end
print(make() == make())
local fs = {}
for i = 1, 2 do table.insert(fs, function() return 1 end) end
print(fs[1] == fs[2])
