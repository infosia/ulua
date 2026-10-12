// D17 uses the same diagnostic predicate as tools/witness/corpus.mjs.
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const [root, directory, label] = process.argv.slice(2);
const ts = require(process.env.TSC_JS || path.join(root, "node_modules/typescript/lib/typescript.js"));
const config = ts.getParsedCommandLineOfConfigFile(path.join(root, "tsconfig.json"), {}, {
  ...ts.sys, onUnRecoverableConfigFileDiagnostic: d => { throw Error(ts.flattenDiagnosticMessageText(d.messageText, "\n")); },
});
if (config.errors.length) throw Error("tsconfig errors");
const files = fs.readdirSync(directory).filter(f => /^case-\d+\.ts$/.test(f)).map(f => path.join(directory, f));
const prelude = config.fileNames.filter(f => f.includes(`${path.sep}prelude${path.sep}`));
const program = ts.createProgram([...prelude, ...files], config.options);
const checker = program.getTypeChecker();
const diagnostics = ts.getPreEmitDiagnostics(program);
const codes = new Map();
const affected = new Set();
const randomFiles = new Set(files.filter(f => {
  const source = fs.readFileSync(f, "utf8");
  return source.startsWith("function identity(") || source.startsWith("// D18 random\n");
}));
const randomCodes = new Map();
const randomAffected = new Set();
for (const d of diagnostics) {
  if (!isAcceptedEquality(d, checker)) throw Error(ts.formatDiagnosticsWithColorAndContext([d], {
    getCanonicalFileName: f => f, getCurrentDirectory: () => root, getNewLine: () => "\n",
  }));
  codes.set(d.code, (codes.get(d.code) || 0) + 1);
  affected.add(d.file.fileName);
  if (randomFiles.has(d.file.fileName)) {
    randomCodes.set(d.code, (randomCodes.get(d.code) || 0) + 1);
    randomAffected.add(d.file.fileName);
  }
}
// Unrelated type errors and mixed comparisons cannot use the exception.
const invalid = path.join(directory, "invalid.ts");
fs.writeFileSync(invalid, "function take(n:number):void{}export function main():void{const wrong:number='x';take('x');const mixed=1==='1';const allowed=1===2;const s='a';if(s==='b'){let copy=s;copy='c';const xs=[s];xs.push('c');}}");
const invalidProgram = ts.createProgram([...prelude, invalid], config.options);
const invalidChecker = invalidProgram.getTypeChecker();
const invalidDiagnostics = ts.getPreEmitDiagnostics(invalidProgram);
const denied = invalidDiagnostics.filter(d => !isAcceptedEquality(d, invalidChecker)).map(d => d.code).sort();
if (denied.join(',') !== '2322,2345,2367' || invalidDiagnostics.filter(d => isAcceptedEquality(d, invalidChecker)).length !== 4)
  throw Error('Equality exception concealed another error');
let node = 0;
for (const file of files) {
  const source = fs.readFileSync(file, "utf8");
  const js = ts.transpileModule(source, {compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
  const output = [];
  const exports = {};
  new Function("exports", "print", js)(exports, s => output.push(s));
  exports.main();
  const actual = output.map(s => s+"\n").join("");
  const expected = fs.readFileSync(file.replace(/\.ts$/, ".expected"), "utf8");
  if (actual !== expected) throw Error(`${file}\n${source}\nNode: ${actual}\nUlua: ${expected}`);
  node++;
}
console.log(`${label} witnesses: node=${node} tsc=${files.length} diagnostic_programs=${affected.size} codes=${JSON.stringify([...codes].sort())} other_codes=[]`);
console.log(`${label} random tsc: programs=${randomFiles.size} diagnostic_programs=${randomAffected.size} codes=${JSON.stringify([...randomCodes].sort())} other_codes=[]`);
console.log(`${label} exception control: ordinary TS2322/TS2345 and mixed TS2367 denied; never TS2322/TS2345 allowed`);

function isAcceptedEquality(d, checker) {
  if (d.code === 2322 || d.code === 2345) return /'never'/.test(ts.flattenDiagnosticMessageText(d.messageText, "\n"));
  if ((d.code !== 2367 && d.code !== 2845) || !d.file || d.start === undefined) return false;
  const kinds = new Set([
    ts.SyntaxKind.EqualsEqualsEqualsToken,
    ts.SyntaxKind.ExclamationEqualsEqualsToken,
    ts.SyntaxKind.EqualsEqualsToken,
    ts.SyntaxKind.ExclamationEqualsToken,
  ]);
  let node = null;
  const visit = (n) => {
    if (n.getStart(d.file) <= d.start && d.start + d.length <= n.getEnd()) {
      if (ts.isBinaryExpression(n) && kinds.has(n.operatorToken.kind)) node = n;
      ts.forEachChild(n, visit);
    }
  };
  visit(d.file);
  if (!node || node.getStart(d.file) !== d.start || node.getEnd() !== d.start + d.length) return false;
  const kind = (e) => {
    const t = checker.getBaseTypeOfLiteralType(checker.getTypeAtLocation(e));
    if (t.flags & ts.TypeFlags.NumberLike) return "number";
    if (t.flags & ts.TypeFlags.StringLike) return "string";
    if (t.flags & ts.TypeFlags.BooleanLike) return "boolean";
    return null;
  };
  const left = kind(node.left);
  return left !== null && left === kind(node.right);
}
