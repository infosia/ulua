local Tree = {}
function Tree.new(depth)
    local node = {left = nil, right = nil}
    if depth > 0 then node.left = Tree.new(depth - 1); node.right = Tree.new(depth - 1) end
    return node
end
local function walk(node)
    if node == nil then return 0 end
    return (1 + walk(node.left) + walk(node.right)) % 4294967296
end
local function main()
    local sum = 0
    for i = 1, 12 do local root = Tree.new(14); sum = (sum + walk(root)) % 4294967296 end
    return sum
end

return main
