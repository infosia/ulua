print(0xffffffffffffffffff == 4722366482869645213696)
print((loadstring("return 0o17")) ~= nil)
print((loadstring("return \"a\rb\"")) ~= nil, (loadstring("return \"a\0b\"")) ~= nil)
-- A long bracket string with a raw CR LF inside, compiled from source.
print(#(loadstring("return [[a\r\nb]]") :: any)(), #"a\r\nb")
