// Regenerate with Node v24.18.0; identity rows are implicit, covering every scalar.
import {writeFileSync} from 'node:fs';
if (process.version !== 'v24.18.0' || process.versions.unicode !== '17.0') {
  throw new Error(`Expected Node v24.18.0 / Unicode 17.0; got ${process.version} / ${process.versions.unicode}`);
}
const hex = s => Buffer.from(s).toString('hex');
const rows = [`# Node ${process.version}; Unicode ${process.versions.unicode}; every scalar; unchanged mappings implicit.`];
for (let cp = 0; cp <= 0x10ffff; cp++) {
  if (cp >= 0xd800 && cp <= 0xdfff) continue;
  const s = String.fromCodePoint(cp), upper = s.toUpperCase(), lower = s.toLowerCase();
  if (upper !== s || lower !== s) rows.push([cp.toString(16), hex(upper), hex(lower)].join('\t'));
}
for (const s of ['ΟΣ','ΟΣΑ','Ο\u0301Σ','ΟΣ\u0301','Σ','AΣ A','İ','ﬀ','ǅ']) {
  rows.push(['@'+hex(s), hex(s.toUpperCase()), hex(s.toLowerCase())].join('\t'));
}
writeFileSync(new URL('string-case-oracle.tsv', import.meta.url), rows.join('\n')+'\n');
console.log(`${rows.length-1} rows written (${process.version})`);
