// Run with node v24.18.0. The seed fixes every input and expected JSON text.
import {writeFileSync} from 'node:fs';
let seed = 0x76c20;
function random() { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 2**32; }
const edges = [-0, Number.MIN_VALUE, Number.MAX_VALUE, 1e21, 1e-7, 0.1 + 0.2, 9007199254740991];
function number() { return random() < 0.3 ? edges[Math.floor(random()*edges.length)] : (random() - 0.5) * 10**Math.floor(random()*40-20); }
const chars = ['a', '"', '\\', '/', '\b', '\t', '\n', '\f', '\r', '\x01', '\x1f', '\x7f', 'é', '😀', '\u2028', '\u2029'];
function string() { return Array.from({length:Math.floor(random()*12)}, () => chars[Math.floor(random()*chars.length)]).join(''); }
function array(f) { return Array.from({length:Math.floor(random()*5)},f); }
const r0 = () => ({n:number()});
const r1 = () => ({s:string(),ok:random()<0.5});
const r2 = () => ({value:random()<0.5?null:number(),label:string()});
const r3 = () => ({point:r0(),items:array(number)});
const types = ['R0','R1','R2','R3','number[]','string[]','R1[]','number[][]','R4','R5'];
const factories = [r0,r1,r2,r3,()=>array(number),()=>array(string),()=>array(r1),()=>array(()=>array(number)),()=>({rows:array(r3),flag:random()<0.5,opt:random()<0.5?null:r1()}),()=>({a:array(()=>random()<0.5),b:array(r2),c:array(()=>array(string))})];
function literal(v) {
    if (Array.isArray(v)) return `[${v.map(literal).join(',')}]`;
    if (v !== null && typeof v === 'object') return `{${Object.entries(v).map(([k,v])=>`${k}:${literal(v)}`).join(',')}}`;
    return Object.is(v,-0) ? '-0' : JSON.stringify(v);
}
let source = 'type R0={n:number};type R1={s:string;ok:boolean};type R2={value:number|null;label:string};type R3={point:R0;items:number[]};type R4={rows:R3[];flag:boolean;opt:R1|null};type R5={a:boolean[];b:R2[];c:string[][]};\n';
let table = `# node ${process.version}; seed 0x76c20; type\tindex\tJSON UTF-8 hex\n`;
for(let t=0;t<types.length;t++) {
    const values = Array.from({length:32}, factories[t]);
    source += `const v${t}:${types[t]}[]=[${values.map(literal).join(',')}];\n`;
    source += `export function s${t}(i:number):string{return JSON.stringify(v${t}[i]);}\n`;
    source += `export function p${t}(text:string):string{const v:${types[t]}=JSON.parse(text);return JSON.stringify(v);}\n`;
    values.forEach((v,i)=>table+=`${t}\t${i}\t${Buffer.from(JSON.stringify(v)).toString('hex')}\n`);
}
writeFileSync(new URL('./json-oracle.ts',import.meta.url),source);
writeFileSync(new URL('./json-oracle.tsv',import.meta.url),table);
console.log(`JSON oracle: ${types.length} types, 320 values, source ${Buffer.byteLength(source)} bytes, ${process.version}`);
