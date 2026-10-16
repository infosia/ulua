local Particle = {}
function Particle.new(i)
    return {x = i % 100, y = i % 71, vx = i % 13 + 1, vy = i % 17 + 1}
end
local function main()
    local a = {}
    for i = 0, 19999 do a[i + 1] = Particle.new(i) end
    for k = 1, 200 do
        for i = 1, 20000 do local p = a[i]; p.x = p.x + p.vx * 0.125; p.y = p.y + p.vy * 0.125 end
    end
    local sum = 0
    for i = 1, 20000 do local p = a[i]; sum = (sum + (p.x * 8 - (p.x * 8) % 1) + (p.y * 8 - (p.y * 8) % 1)) % 4294967296 end
    return sum
end

return main
