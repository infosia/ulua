// corpus: accept/a68-native-remainder-edges
// purpose: Remainders at the edges of the integer facts: 2^53 saturation, -0, non-integers, infinite and negative operands.
// rule: C4
// phase: P6
// tsc: accepts
// js-comparable: yes
function saturated(): string {
  let x = 9007199254740990;
  x++;
  x++;
  x++;
  x++;
  const big = 9007199254740992;
  return `${x === big} ${x % 7} ${x % 1000000007} ${5 % big} ${x % big}`;
}

function negativeZero(): string {
  let z = 0;
  z = z * -1;
  return `${1 / (z % 5)} ${1 / (0 % 5)}`;
}

function nonInteger(): string {
  let h = 7;
  h = h / 2;
  const t = 7;
  const inf = 1 / 0;
  return `${h % 2} ${t % 0.1} ${t % inf}`;
}

function negative(): string {
  const s = 10;
  const u = 3;
  let d = 3;
  d--;
  d--;
  d--;
  d--;
  let c = 5;
  const reset = (): void => {
    c = -4;
  };
  reset();
  return `${(s - u) % 4} ${(u - s) % 4} ${d % 2} ${c % 3}`;
}

function product(): string {
  let p = 94906267;
  p = p * p;
  let q = 4096;
  q = q * q;
  return `${p % 10} ${q % 1000}`;
}

function chains(): string {
  let state = 1;
  let count = 0;
  let sum = 0;
  for (let i = 0; i < 1000; i++) {
    state = (state * 1664525 + 1013904223) % 4294967296;
    count = (count + 1) % 3;
    sum = (sum + state % 1000 + count) % 4294967296;
  }
  let n = 0;
  let d = 2;
  for (let k = 2; k <= 50; k++) {
    d = 2;
    while (d * d <= k && k % d !== 0) {
      d++;
    }
    if (d * d > k) {
      n++;
    }
  }
  return `${state} ${count} ${sum} ${n}`;
}

export function main(): void {
  print(saturated());
  print(negativeZero());
  print(nonInteger());
  print(negative());
  print(product());
  print(chains());
}
