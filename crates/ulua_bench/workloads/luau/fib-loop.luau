local function main()
    local sum = 0
    for i = 0, 999999 do
        local a, b = 0, 1
        for j = 1, i % 40 do local next = a + b; a = b; b = next end
        sum = (sum + a) % 4294967296
    end
    return sum
end

return main
