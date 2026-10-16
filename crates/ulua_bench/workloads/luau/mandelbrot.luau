local function main()
    local sum = 0
    for row = 0, 399 do
        local cy = row / 400 * 2 - 1
        for col = 0, 399 do
            local cx = col / 400 * 3 - 2
            local x, y, count = 0, 0, 0
            while count < 255 and x*x + y*y < 4 do
                local next = x*x - y*y + cx
                y = 2*x*y + cy; x = next; count = count + 1
            end
            sum = (sum + count) % 4294967296
        end
    end
    return sum
end

return main
