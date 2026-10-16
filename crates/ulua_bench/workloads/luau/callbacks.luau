local function main()
    local a, state, sum = {}, 1, 0
    for i = 0, 99999 do state = (state * 1664525 + 1013904223) % 4294967296; a[i + 1] = state end
    for k = 0, 19 do
        local mapped, kept = {}, {}
        for i = 0, 99999 do mapped[i + 1] = (a[i + 1] + i + k) % 4294967296 end
        for i = 0, 99999 do if (mapped[i + 1] + i) % 3 ~= 0 then kept[#kept + 1] = mapped[i + 1] end end
        local round = 0
        for i = 0, #kept - 1 do round = (round + kept[i + 1] + i) % 4294967296 end
        sum = (sum + round) % 4294967296
    end
    return sum
end

return main
