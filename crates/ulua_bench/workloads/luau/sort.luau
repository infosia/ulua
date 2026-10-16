local function quick(a, lo, hi)
    local i, j = lo, hi
    local pivot = a[math.floor((lo + hi) / 2)]
    while i <= j do
        while a[i] < pivot do i = i + 1 end
        while a[j] > pivot do j = j - 1 end
        if i <= j then local v = a[i]; a[i] = a[j]; a[j] = v; i = i + 1; j = j - 1 end
    end
    if lo < j then quick(a, lo, j) end
    if i < hi then quick(a, i, hi) end
end
local function main()
    local a, state, sum = {}, 1, 0
    for i = 1, 100000 do
        state = (state * 1664525 + 1013904223) % 4294967296; a[i] = state
    end
    quick(a, 1, 100000)
    for i = 1, 100000, 97 do sum = (sum + a[i]) % 4294967296 end
    return sum
end

return main
