local n = 0
local function inc(): number
	n += 1
	return n
end
print(n + inc())
