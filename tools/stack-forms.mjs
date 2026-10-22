// Measure the compiler stack peak of each nesting form at its largest accepted depth.
// Usage: LUAU_SOURCE_DIR=<Luau checkout> node tools/stack-forms.mjs
// It builds s3_measure (feature `measurements`) in Debug and Release.
// For each form, it finds the largest accepted depth (1 to 64) with the Release `check` mode.
// Then it runs the `source` mode at that depth in both builds.
// `s3_measure stack --half` covers the P1 forms; this script covers the later forms.
// It exits 1 when a form gives `internal`, a peak is zero, or a peak is above half of COMPILER_STACK_BYTES.
// Measured cost on macOS arm64: 22 s with both builds up to date (32 forms).
// Not a gate step: pipeline.md §7 item 1 asks for this run on each measured target.
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const MAX_DEPTH = 64;

const rep = (open, mid, close, n) => open.repeat(n) + mid + close.repeat(n);
const C = 'class C{m(x:number):number{return x}}';

// Each form maps a depth to a source. Every form type-checks at depth 1.
const FORMS = {
  'method-stmt': (n) => `${C}function f(c:C):void{${rep('c.m(', '1', ')', n)}}`,
  'method-return': (n) => `${C}function f(c:C):number{return ${rep('c.m(', '1', ')', n)}}`,
  'math-return': (n) => `function f():number{return ${rep('Math.abs(', '1', ')', n)}}`,
  'math-max-return': (n) => `function f():number{return ${rep('Math.max(1,', '1', ')', n)}}`,
  'call-stmt': (n) => `function g(x:number):number{return x}function f():void{${rep('g(', '1', ')', n)}}`,
  'array-literal': (n) => `function f():void{let a=${rep('[', '1', ']', n)}}`,
  'array-index': (n) => `function f(a:number[]):number{return ${rep('a[', '0', ']', n)}}`,
  'record-literal': (n) => `type R={a:number};function f():void{let r=${rep('{a:', '1', '}.a', n)}}`,
  'string-method': (n) => `function f(s:string):string{return s${'.slice(0)'.repeat(n)}}`,
  'char-code': (n) => `function f(s:string):number{return ${rep('s.charCodeAt(', '0', ')', n)}}`,
  'template-nested': (n) => `function f():string{return ${rep('`a${', '1', '}`', n)}}`,
  'logical-and': (n) => `function f(b:boolean):boolean{return ${rep('(b&&', 'b', ')', n)}}`,
  'logical-chain': (n) => `function f():boolean{return ${'true&&'.repeat(n)}true}`,
  'nullish': (n) => `function f(x:number|null):number{return ${rep('(x??', '1', ')', n)}}`,
  'negation': (n) => `function f(x:number):number{return ${rep('-(', 'x', ')', n)}}`,
  'arrow-call': (n) => `function f():number{return ${rep('((x:number):number=>', 'x', ')(1)', n)}}`,
  'closure-block': (n) => `function f():void{${rep('let g=():void=>{', '', '};g();', n)}}`,
  'function-decl': (n) => `function f():void{${rep('function g():void{', '', '}', n)}}`,
  'for-of': (n) => `function f(a:number[]):number{let s=0;${rep('for(const x of a){', 's+=x;', '}', n)}return s}`,
  'for-of-return': (n) => `function f(a:number[]):number{${rep('for(const x of a){', 'return x;', '}', n)}return 0}`,
  'try-catch': (n) => `function f():void{${rep('try{', '', '}catch(e){}', n)}}`,
  'try-finally': (n) => `function f():void{${rep('try{', '', '}finally{}', n)}}`,
  'catch-body': (n) => `function f():void{${rep('try{}catch(e){', '', '}', n)}}`,
  'try-in-for': (n) => `function f(a:number[]):void{${rep('for(const x of a){try{', '', '}catch(e){}}', n)}}`,
  'map-callback': (n) => `function f(a:number[]):number{return ${rep('a.map((x:number):number=>x+', '1', ').length', n)}}`,
  'map-get': (n) => `function f(m:Map<number,number>):number{return ${rep('(m.get(', '1', ')??0)', n)}}`,
  'class-new': (n) => `class A{x:number;constructor(x:number){this.x=x}}function f():number{return ${rep('new A(', '1', ').x', n)}}`,
  'async-await': (n) => `async function g(x:number):Promise<number>{return x}async function f():Promise<number>{return ${rep('await g(', '1', ')', n)}}`,
  'json-stringify': (n) => `function f():string{return ${rep('JSON.stringify(', '1', ')', n)}}`,
  'narrow-if': (n) => `function f(x:number|null):number{${rep('if(x!==null){', 'return x;', '}', n)}return 0}`,
  'conditional-chain': (n) => `function f(x:number):number{return ${rep('x>0?', '1', ':0', n)}}`,
  'else-if-chain': (n) => `function f(x:number):number{if(x===0){return 0}${'else if(x===1){return 1}'.repeat(n)}return 2}`,
};

const exe = process.platform === 'win32' ? '.exe' : '';
const binary = (profile) => join(root, 'target', profile, 'examples', `s3_measure${exe}`);
const run = (bin, args) => {
  try {
    return execFileSync(bin, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  } catch (e) {
    return `${e.stdout ?? ''}${e.stderr ?? ''}exit ${e.status}`;
  }
};

for (const release of [false, true]) {
  const args = ['build', '-p', 'ulua', '--features', 'measurements', '--example', 's3_measure'];
  if (release) args.splice(1, 0, '--release');
  execFileSync('cargo', args, { cwd: root, stdio: 'inherit' });
}

const dir = mkdtempSync(join(tmpdir(), 'ulua-stack-forms-'));
const rows = [];
let failed = false;
try {
  for (const [name, make] of Object.entries(FORMS)) {
    let depth = 0;
    let next = '';
    for (let n = 1; n <= MAX_DEPTH; n++) {
      const file = join(dir, `${name}-${n}.ts`);
      writeFileSync(file, make(n));
      const status = run(binary('release'), ['check', file]).trim().split('\n').at(-1);
      if (status !== 'check ok') {
        next = status;
        break;
      }
      depth = n;
    }
    if (next.includes('internal') || depth === 0) failed = true;
    const peaks = {};
    for (const profile of ['debug', 'release']) {
      if (depth === 0) break;
      const out = run(binary(profile), ['source', join(dir, `${name}-${depth}.ts`)]);
      const margin = out.match(/margin peak=(\d+) half_stack=(\w+)/);
      // A peak of zero means that the target has no stack watermark (pipeline.md §7 item 1).
      if (!margin || margin[1] === '0' || margin[2] !== 'true' || !/status=ok/.test(out)) failed = true;
      peaks[profile] = margin ? Number(margin[1]) : NaN;
    }
    rows.push({ name, depth, next, ...peaks });
  }
} finally {
  rmSync(dir, { recursive: true, force: true });
}

console.log('| Form | Depth | Next | Debug | Release |');
console.log('|---|---:|---|---:|---:|');
for (const r of rows) {
  console.log(`| ${r.name} | ${r.depth} | ${r.next.replace(/^check /, '')} | ${r.debug} | ${r.release} |`);
}
const top = (key) => rows.reduce((a, b) => ((b[key] ?? 0) > (a[key] ?? 0) ? b : a));
console.log(`largest debug: ${top('debug').name} ${top('debug').debug}`);
console.log(`largest release: ${top('release').name} ${top('release').release}`);
console.log(failed ? 'stack-forms: FAIL' : 'stack-forms: PASS');
process.exit(failed ? 1 : 0);
