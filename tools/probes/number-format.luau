-- Number to string over a seeded sample. Each line: category and tostring(v).
-- The .mjs pair prints the same values; compare the two outputs line by line.
-- With FORMAT set to "lowered", each value goes through the candidate lowering.
local x = 2463534242
local function rnd(): number
	x = bit32.bxor(x, bit32.lshift(x, 13))
	x = bit32.bxor(x, bit32.rshift(x, 17))
	x = bit32.bxor(x, bit32.lshift(x, 5))
	return x
end
local buf = buffer.create(8)
local pow10 = { 1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22 }

local function lowered(n: number): string
	if n ~= n then
		return "NaN"
	elseif n == math.huge then
		return "Infinity"
	elseif n == -math.huge then
		return "-Infinity"
	elseif n == 0 then
		return "0"
	end
	local s = string.gsub(tostring(n), "e([+-])0*(%d)", "e%1%2")
	return s
end

local fmt = if (...) == "lowered" then lowered else tostring
local N = 20000
local out = {}
for i = 0, N - 1 do
	buffer.writeu32(buf, 0, rnd())
	buffer.writeu32(buf, 4, rnd())
	table.insert(out, "bits " .. fmt(buffer.readf64(buf, 0)))
end
for i = 0, N - 1 do
	local v = rnd() * 2097152 + (rnd() % 2097152)
	table.insert(out, "int " .. fmt(if i % 2 == 0 then v else -v))
end
for i = 0, N - 1 do
	local v = (rnd() % 100000) / pow10[rnd() % 13 + 1]
	table.insert(out, "dec " .. fmt(v))
end
for i = 0, N - 1 do
	local m = rnd() / 4294967296 * 9 + 1
	local e = rnd() % 10
	local v = if e < 5 then m / pow10[5 + e + 1] else m * pow10[13 + e + 1]
	table.insert(out, "edge " .. fmt(v))
end
local specials = { 0, -0, 0 / 0, math.huge, -math.huge, 5e-324, -5e-324, 1.7976931348623157e308, 1e21, 999999999999999900000, 1e-7, 1e-6, 0.000001, 1.5e-7, 2 ^ 53, 2 ^ 53 + 2, -1e21, -1e-7, 2.2250738585072014e-308 }
for _, v in specials do
	table.insert(out, "special " .. fmt(v))
end
print(table.concat(out, "\n"))
