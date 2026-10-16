local Cell = {}
Cell.__index = Cell
function Cell.new(i, next)
    local cell = setmetatable({id = i, text = "", values = {}, next = next}, Cell)
    for j = 1, 1 + i % 31 do cell.text = cell.text .. "x" end
    for j = 0, i % 7 do cell.values[j + 1] = i + j end
    return cell
end
local function main()
    local sum = 0
    for k = 1, 4 do
        local all, kept, previous = {}, {}, nil
        for i = 0, 39999 do
            local cell = Cell.new(i, previous); all[i + 1] = cell
            if i % 2 == 0 then previous = cell end
        end
        for i = 1, 40000, 2 do kept[#kept + 1] = all[i] end
        all = {}
        for i = 1, #kept do
            local cell = kept[i]
            sum = (sum + cell.id + #cell.text) % 4294967296
            if cell.next ~= nil then sum = (sum + cell.next.id) % 4294967296 end
            for j = 1, #cell.values do sum = (sum + cell.values[j]) % 4294967296 end
        end
    end
    return sum
end

return main
