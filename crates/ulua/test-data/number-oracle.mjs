// Regenerate with Node; no product code participates in the reference outputs.
import {writeFileSync} from 'node:fs';
const bytes = new ArrayBuffer(8), view = new DataView(bytes);
const bits = x => { view.setFloat64(0,x); return view.getBigUint64(0).toString(16).padStart(16,'0'); };
const fromBits = x => { view.setBigUint64(0,x); return view.getFloat64(0); };
let seed = 0x736c696365703733n;
function random() { seed ^= seed << 13n; seed ^= seed >> 7n; seed ^= seed << 17n; seed = BigInt.asUintN(64,seed); return seed; }
const pick = n => Number(random()%BigInt(n));
const hex = s => Buffer.from(s).toString('hex');
const edge = [NaN, Infinity, -Infinity, 0, -0, Number.MIN_VALUE, -Number.MIN_VALUE, Number.MAX_VALUE, 1e21, 1e-6, 1e-7, 2.5, -2.5, 0.25, 1.005, 1234.5678, 0.1, 9007199254740992];
const rows = [`# Node ${process.version}; seeded xorshift64; input/output bit patterns preserve signed zero.`];
for(let i=0;i<10100;i++) {
  let x = i<edge.length ? edge[i] : fromBits(random());
  if(i%4===0 && i>=edge.length) x = (Number(random()>>11n)/2**53-0.5)*10**(pick(40)-20);
  const n=pick(101), p=1+pick(100), r=2+pick(35);
  for(const [op,arg,output] of [
    [2,n,x.toFixed(n)],[3,n,x.toExponential(n)],[4,p,x.toPrecision(p)],[5,r===10?36:r,x.toString(r===10?36:r)],[5,10,x.toString(10)],
    [3,'_',x.toExponential()],[2,'_',x.toFixed()],
    [6,0,Number.isNaN(x)],[7,0,Number.isFinite(x)],[8,0,Number.isInteger(x)],[9,0,Number.isSafeInteger(x)]
  ]) rows.push([op,bits(x),arg,hex(String(output))].join('\t'));
  let s = ['',' ','\u00a0\ufeff','\u0085','\n\u2000'][pick(5)] + ['','+','-'][pick(3)];
  const alphabet='0123456789abcdefghijklmnopqrstuvwxyz';
  let digits=''; for(let j=pick(300);j>0;j--) digits+=alphabet[pick(r)];
  s+=digits+['','!',' xyz','e+4'][pick(4)];
  // The exact integer oracle is separate from Node parseInt's radix accumulation.
  const trimmed=s.trimStart(); let t=trimmed.replace(/^[+-]/,'');
  if(r===16) t=t.replace(/^0[xX]/,'');
  let big=0n, any=false;
  for(const c of t.toLowerCase()) { const d=alphabet.indexOf(c); if(d<0||d>=r) break; big=big*BigInt(r)+BigInt(d); any=true; }
  let exact=any?Number(big):NaN; if(trimmed.startsWith('-')) exact=-exact;
  rows.push([0,hex(s),r,bits(parseInt(s,r)),bits(exact)].join('\t'));
  const f = ['','\ufeff\n','\u0085'][pick(3)]+['','+','-'][pick(3)]+(i%11===0?'Infinity':`${pick(100000000)}.${pick(100000000)}e${pick(800)-400}`)+['','junk','e+','x'][pick(4)];
  rows.push([1,hex(f),0,bits(parseFloat(f))].join('\t'));
}
writeFileSync(new URL('number-oracle.tsv',import.meta.url),rows.join('\n')+'\n');
console.log(`${rows.length-1} rows written (${process.version})`);
