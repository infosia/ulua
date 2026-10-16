local function place(cols, row)
    if row == 11 then return 1 end
    local count = 0
    for col = 0, 10 do
        local safe = true
        for r = 0, row - 1 do
            if cols[r + 1] == col or cols[r + 1] - col == row - r or col - cols[r + 1] == row - r then safe = false; break end
        end
        if safe then cols[row + 1] = col; count = (count + place(cols, row + 1)) % 4294967296 end
    end
    return count
end
local function main()
    local cols = {}
    for i = 1, 11 do cols[i] = 0 end
    return place(cols, 0)
end

return main
