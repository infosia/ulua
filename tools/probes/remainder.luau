local function show(v: number): string
	if v == 0 and 1 / v < 0 then return "-0" end
	if v ~= v then return "NaN" end
	if v == math.huge then return "Infinity" end
	if v == -math.huge then return "-Infinity" end
	return tostring(v)
end
local cases = { { -7, 3 }, { 7, -3 }, { -4, 2 }, { 5.5, 2 }, { 5, 0 }, { math.huge, 2 }, { 2, math.huge }, { -2, math.huge } }
for _, c in cases do
	local a, b = c[1], c[2]
	print(`{show(a)} % {show(b)} = {show(a % b)} (fmod {show(math.fmod(a, b))})`)
end
