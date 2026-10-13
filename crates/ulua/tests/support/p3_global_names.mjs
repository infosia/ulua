// Read global value and type-only names from the ES2022 library files and the Ulua prelude.
import ts from './node_modules/typescript/lib/typescript.js';
const program = ts.createProgram(['prelude/ulua.d.ts'], {
  strict: true,
  target: ts.ScriptTarget.ES2022,
  lib: ['lib.es2022.d.ts'],
});
const prelude = program.getSourceFile('prelude/ulua.d.ts');
const checker = program.getTypeChecker();
const names = checker.getSymbolsInScope(prelude, ts.SymbolFlags.Value);
const types = checker.getSymbolsInScope(prelude, ts.SymbolFlags.Type)
  .filter(symbol => !(symbol.flags & ts.SymbolFlags.Value));
const sorted = symbols => symbols.map(symbol => symbol.name).sort().join('\n');
process.stdout.write(`${sorted(names)}\n\n${sorted(types)}`);
