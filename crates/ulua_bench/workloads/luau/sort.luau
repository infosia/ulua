local function quick(a, lo, hi)
    local i, j = lo, hi
    local pivot = a[(lo + hi - (lo + hi) % 2) / 2 + 1]
    while i <= j do
        while a[i + 1] < pivot do i = i + 1 end
        while a[j + 1] > pivot do j = j - 1 end
        if i <= j then local v = a[i + 1]; a[i + 1] = a[j + 1]; a[j + 1] = v; i = i + 1; j = j - 1 end
    end
    if lo < j then quick(a, lo, j) end
    if i < hi then quick(a, i, hi) end
end
local function main()
    local a, state, sum = {}, 1, 0
    for i = 0, 99999 do
        state = (state * 1664525 + 1013904223) % 4294967296; a[i + 1] = state
    end
    quick(a, 0, 99999)
    for i = 0, 99999, 97 do sum = (sum + a[i + 1]) % 4294967296 end
    return sum
end

return main
