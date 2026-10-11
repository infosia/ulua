local function show(v: number): string
	if v ~= v then return "NaN" end
	if v == math.huge then return "Infinity" end
	if v == -math.huge then return "-Infinity" end
	return tostring(v)
end
local cases = { { 1, 0 / 0 }, { 0 / 0, 0 }, { -8, 1 / 3 }, { 2, 0.5 }, { -1, math.huge }, { 1, math.huge }, { 0, -1 }, { -0, -1 } }
for _, c in cases do
	print(`{show(c[1])} ** {show(c[2])} = {show(c[1] ^ c[2])}`)
end
