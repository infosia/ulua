// The committed Node fixtures supply the independent host reference.
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
const [root, directory] = process.argv.slice(2);
const require = createRequire(import.meta.url);
const ts = require(path.join(root, 'node_modules/typescript/lib/typescript.js'));
const { install, FIXTURE_NAMES } = await import(pathToFileURL(path.join(root, 'tools/witness/node-prelude.mjs')));
let count = 0;
for (const name of fs.readdirSync(directory).filter(name => /^case-\d+\.ts$/.test(name))) {
  const file = path.join(directory, name);
  const source = fs.readFileSync(file, 'utf8');
  const js = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText;
  const output = [];
  install(output, FIXTURE_NAMES);
  const exports = {};
  new Function('exports', js)(exports);
  exports.main();
  const expected = fs.readFileSync(file.replace(/\.ts$/, '.expected'), 'utf8');
  if (output.join('\n') !== expected) throw Error(`${name}\n${source}\nnode=${output.join('\n')}\nulua=${expected}`);
  count++;
}
console.log(`P3 Node reference programs=${count} fixtures=${FIXTURE_NAMES.length}`);
