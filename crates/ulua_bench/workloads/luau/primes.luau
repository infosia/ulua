local function main()
    local count = 0
    for n = 2, 200000 do
        local prime = true
        local d = 2
        while d*d <= n do
            if n % d == 0 then prime = false; break end
            d = d + 1
        end
        if prime then count = (count + 1) % 4294967296 end
    end
    return count
end

return main
