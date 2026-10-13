// Read actual global values from the ES2022 library files and the Ulua prelude.
import ts from './node_modules/typescript/lib/typescript.js';
const program = ts.createProgram(['prelude/ulua.d.ts'], {
  strict: true,
  target: ts.ScriptTarget.ES2022,
  lib: ['lib.es2022.d.ts'],
});
const prelude = program.getSourceFile('prelude/ulua.d.ts');
const names = program.getTypeChecker().getSymbolsInScope(prelude, ts.SymbolFlags.Value);
process.stdout.write(names.map(symbol => symbol.name).sort().join('\n'));
