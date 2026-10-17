// Regenerate: node crates/ulua/test-data/map-oracle.mjs
import fs from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const ts = require('typescript');
Map.prototype.getOr = function(k, fallback) { return this.has(k) ? this.get(k) : fallback; };
const source = fs.readFileSync(new URL('./map-oracle.ts', import.meta.url), 'utf8');
const js = ts.transpileModule(source, {compilerOptions: {target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS}}).outputText;
const exports = {};
new Function('exports', js)(exports);
const lines = [];
for (let seed = 1; seed <= 10_000; seed++) lines.push(`${seed}\t${exports.main(seed)}`);
fs.writeFileSync(new URL('./map-oracle.tsv', import.meta.url), lines.join('\n') + '\n');
console.log(`Node ${process.version}: ${lines.length} sequences, 32 operations/sequence`);
