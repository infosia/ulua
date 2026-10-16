local function main()
    local a, state, sum = {}, 1, 0
    for i = 1, 100000 do state = (state * 1664525 + 1013904223) % 4294967296; a[i] = state end
    for k = 0, 19 do
        local mapped, kept = {}, {}
        for i = 1, 100000 do mapped[i] = (a[i] + i - 1 + k) % 4294967296 end
        for i = 1, 100000 do if (mapped[i] + i - 1) % 3 ~= 0 then kept[#kept + 1] = mapped[i] end end
        local round = 0
        for i = 1, #kept do round = (round + kept[i] + i - 1) % 4294967296 end
        sum = (sum + round) % 4294967296
    end
    return sum
end

return main
