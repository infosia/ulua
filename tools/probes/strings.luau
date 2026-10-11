print(#"héllo", #"😀", #"日本")
print("\u{FFFF}" < "😀", "z" < "é")
print(string.upper("é"), string.upper("straße"), string.lower("ÀB"))
print((string.gsub(" \u{a0}x\u{3000}", "^%s+", "")))
print(table.concat(string.split("a,b,,c", ","), "|"), table.concat(string.split("日本語", "本"), "|"), #string.split("hé", ""))
local s = "héllo"
print(string.find(s, "ll", 1, true) ~= nil, string.find(s, "", 1, true) ~= nil, string.sub(s, 1, #"hé") == "hé", string.sub(s, -#"llo") == "llo", string.sub(s, 3, 3) == "l")
print("a" .. 1e-7, "a" .. -0)
