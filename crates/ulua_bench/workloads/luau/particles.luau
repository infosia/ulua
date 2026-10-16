local Particle = {}
Particle.__index = Particle
function Particle.new(i)
    return setmetatable({x = i % 100, y = i % 71, vx = i % 13 + 1, vy = i % 17 + 1}, Particle)
end
local function main()
    local a = {}
    for i = 0, 19999 do a[i + 1] = Particle.new(i) end
    for k = 1, 200 do
        for i = 1, 20000 do local p = a[i]; p.x = p.x + p.vx * 0.125; p.y = p.y + p.vy * 0.125 end
    end
    local sum = 0
    for i = 1, 20000 do local p = a[i]; sum = (sum + math.floor(p.x * 8) + math.floor(p.y * 8)) % 4294967296 end
    return sum
end

return main
