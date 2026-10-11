local xs = { 10, 20 }
print(xs[6], xs[0], xs[1.5])
-- The lowered index i + 1 rounds to an integer for these values of i.
print(xs[0.9999999999999999 + 1], xs[-1e-17 + 1])
