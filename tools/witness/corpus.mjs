// Corpus witnesses: header format, tsc, and node (specs/blocks/corpus.md).
// Usage: node tools/witness/corpus.mjs
// TSC_JS names typescript.js; the default is node_modules/typescript.
// Cost: one tsc program per corpus group, and one in-process import per
// accept or trap entry.

import { readFileSync, readdirSync, existsSync, mkdtempSync, writeFileSync, rmSync } from "node:fs";
import os from "node:os";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { install, FIXTURE_NAMES, declarations } from "./node-prelude.mjs";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "../..");
const require = createRequire(import.meta.url);
const tsPath = process.env.TSC_JS || path.join(root, "node_modules/typescript/lib/typescript.js");
if (!existsSync(tsPath)) {
  console.error(`typescript not found at ${path.relative(root, tsPath)}; run npm install or set TSC_JS`);
  process.exit(2);
}
const ts = require(tsPath);

const failures = [];
const fail = (entry, message) => failures.push(`${entry}: ${message}`);

const NAME = /^([art])(\d{2,})-[a-z0-9-]+\.ts$/;
const DIRS = { a: "accept", r: "reject", t: "trap" };
const ORDER = ["corpus", "purpose", "rule", "phase", "tsc"];
// Trap kinds: the table of specs/blocks/corpus.md section 6.
const corpusSpec = readFileSync(path.join(root, "specs/blocks/corpus.md"), "utf8");
const TRAP_KINDS = new Set(
  [...corpusSpec.slice(corpusSpec.indexOf("## 6. Trap kinds")).matchAll(/^\| `([a-z-]+)` \|/gm)].map((m) => m[1]),
);

// Host fixtures: the table of specs/blocks/corpus.md section 7 equals the node implementations.
{
  const names = [...corpusSpec.slice(corpusSpec.indexOf("## 7. Host fixtures")).matchAll(/^\| `([A-Za-z]+)` \|/gm)].map((m) => m[1]);
  if (names.join(",") !== FIXTURE_NAMES.join(","))
    failures.push(`corpus.md section 7 fixtures [${names}] differ from node-prelude.mjs [${FIXTURE_NAMES}]`);
}

function readEntries(dir) {
  const full = path.join(root, "corpus", dir);
  return readdirSync(full)
    .filter((f) => f.endsWith(".ts"))
    .sort()
    .map((file) => ({ dir, file, path: path.join(full, file), id: `${dir}/${file.slice(0, -3)}` }));
}

function parseHeader(entry) {
  const header = {};
  const keys = [];
  for (const line of readFileSync(entry.path, "utf8").split("\n")) {
    const m = /^\/\/ ([a-z-]+): (.*)$/.exec(line);
    if (!m) break;
    header[m[1]] = m[2];
    keys.push(m[1]);
  }
  const want = [...ORDER];
  if (keys.includes("grants")) want.splice(4, 0, "grants");
  if (entry.dir !== "reject") want.push("js-comparable");
  if (entry.dir === "reject") want.push("expected-error");
  if (entry.dir === "trap") want.push("trap");
  if (keys.join(",") !== want.join(",")) fail(entry.id, `header keys ${keys.join(",")}, want ${want.join(",")}`);
  const m = NAME.exec(entry.file);
  if (!m || DIRS[m[1]] !== entry.dir) fail(entry.id, "file name does not match its directory");
  if (header.corpus !== entry.id) fail(entry.id, `corpus header is ${header.corpus}`);
  if (entry.dir !== "reject" && header.tsc !== "accepts" && !/^exception TS\d+(, TS\d+)*$/.test(header.tsc || ""))
    fail(entry.id, "accept and trap entries state tsc: accepts or tsc: exception <codes of invariant 3>");
  if (header.grants !== undefined) {
    const names = header.grants.split(", ");
    if (names.some((n) => !FIXTURE_NAMES.includes(n)) || new Set(names).size !== names.length)
      fail(entry.id, `grants '${header.grants}' must name distinct fixtures of corpus.md section 7`);
  }
  if (!/^P[1-6](\.[1-9])?$/.test(header.phase || "")) fail(entry.id, `phase ${header.phase} is not P1 to P6, with an optional slice`);
  const rules = (header.rule || "").split(", ").filter(Boolean);
  if (entry.dir !== "reject") {
    const jc = /^(yes|no (C\d+))$/.exec(header["js-comparable"] || "");
    if (!jc) fail(entry.id, `js-comparable is '${header["js-comparable"]}', want 'yes' or 'no <collision id>'`);
    else if (jc[2] && !rules.includes(jc[2])) fail(entry.id, `js-comparable cites ${jc[2]}, which the rule header does not name`);
  }
  if (entry.dir === "reject") {
    const id = /^(C\d+): ./.exec(header["expected-error"] || "");
    if (!id || !rules.includes(id[1])) fail(entry.id, "expected-error must start with an id of the rule header");
  }
  if (entry.dir === "trap" && !TRAP_KINDS.has(header.trap)) fail(entry.id, `trap kind ${header.trap} is not in corpus.md section 6`);
  return header;
}

function expectedCodes(tscHeader) {
  if (tscHeader === "accepts") return [];
  const eq = /^exception (TS\d+(?:, TS\d+)*)$/.exec(tscHeader || "");
  if (eq) return eq[1].split(", ").map((c) => Number(c.slice(2)));
  const m = /^errors (TS\d+(?:, TS\d+)*)$/.exec(tscHeader || "");
  return m ? m[1].split(", ").map((c) => Number(c.slice(2))) : null;
}

function tscCodes(files, hostFiles) {
  const config = ts.getParsedCommandLineOfConfigFile(path.join(root, "tsconfig.json"), {}, {
    ...ts.sys,
    onUnRecoverableConfigFileDiagnostic: (d) => {
      throw new Error(ts.flattenDiagnosticMessageText(d.messageText, "\n"));
    },
  });
  for (const d of config.errors) fail("tsconfig.json", ts.flattenDiagnosticMessageText(d.messageText, "\n"));
  const prelude = config.fileNames.filter((f) => f.includes(`${path.sep}prelude${path.sep}`));
  const program = ts.createProgram([...prelude, ...hostFiles, ...files], config.options);
  const byFile = new Map(files.map((f) => [path.resolve(f), new Set()]));
  const atEquality = new Map(files.map((f) => [path.resolve(f), true]));
  const checker = program.getTypeChecker();
  const other = [];
  for (const d of ts.getPreEmitDiagnostics(program)) {
    const key = d.file ? path.resolve(d.file.fileName) : null;
    if (key && byFile.has(key)) {
      byFile.get(key).add(d.code);
      if (!isAcceptedEquality(d, checker)) atEquality.set(key, false);
    } else other.push(ts.flattenDiagnosticMessageText(d.messageText, "\n"));
  }
  return { byFile, atEquality, other };
}

// CLAUDE.md invariant 3, collisions.md C7: TS2367 or TS2845 on an
// equality expression whose operands have one primitive type, with
// literal types widened; or any diagnostic whose message names the
// type `never`, the narrowing that an accepted equality, a null test,
// or an assignment of `null` leaves.
function isAcceptedEquality(d, checker) {
  if (/\b[Tt]ype 'never'/.test(ts.flattenDiagnosticMessageText(d.messageText, "\n"))) return true;
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
    // A contradictory accepted equality narrows an operand to `never`.
    if (t.flags & ts.TypeFlags.Never) return "never";
    if (t.flags & ts.TypeFlags.NumberLike) return "number";
    if (t.flags & ts.TypeFlags.StringLike) return "string";
    if (t.flags & ts.TypeFlags.BooleanLike) return "boolean";
    return null;
  };
  const left = kind(node.left);
  const right = kind(node.right);
  if (left === null || right === null) return false;
  return left === right || left === "never" || right === "never";
}

async function runNode(entry) {
  const output = [];
  const requests = [];
  const grants = headers.get(entry.id).grants;
  install(output, grants === undefined ? [] : grants.split(", "), requests);
  let thrown = null;
  // collisions.md C19.7: node reports an unhandled rejection after the
  // job drain; the runner records the first one as the throw.
  const onRejection = (e) => {
    if (thrown === null) thrown = e;
  };
  process.on("unhandledRejection", onRejection);
  try {
    const mod = await import(pathToFileURL(entry.path).href);
    const result = mod.main();
    let settled = !(result instanceof Promise);
    if (!settled) {
      result.then(
        () => (settled = true),
        (e) => {
          settled = true;
          if (thrown === null) thrown = e;
        },
      );
    }
    // One macrotask lets node finish the drain and report rejections.
    await new Promise((resolve) => setImmediate(resolve));
    // corpus.md section 7: settle the open requests one per macrotask.
    while (requests.length > 0 && thrown === null) {
      requests.shift()();
      await new Promise((resolve) => setImmediate(resolve));
    }
    if (!settled && thrown === null) thrown = new Error("async:pending");
  } catch (e) {
    if (thrown === null) thrown = e;
  }
  process.off("unhandledRejection", onRejection);
  return { output, thrown };
}

const accept = readEntries("accept");
const reject = readEntries("reject");
const trap = readEntries("trap");
const headers = new Map();
for (const e of [...accept, ...reject, ...trap]) headers.set(e.id, parseHeader(e));

// A number is never reused (corpus.md section 1). A retired name keeps
// its number reserved.
const collisionsText = readFileSync(path.join(root, "specs/blocks/collisions.md"), "utf8");
const seen = new Map();
for (const m of collisionsText.matchAll(/retired:([art])(\d{2,})-[a-z0-9-]+/g)) seen.set(`${m[1]}${m[2]}`, `retired:${m[0].slice(8)}`);
for (const e of [...accept, ...reject, ...trap]) {
  const m = NAME.exec(e.file);
  if (!m) continue;
  const key = `${m[1]}${m[2]}`;
  if (seen.has(key)) fail(e.id, `number ${key} is also used by ${seen.get(key)}`);
  seen.set(key, e.id);
}

// Rule citations: each rule in collisions.md names its entries, and each
// entry names its rules. The two lists must agree.
const shortIds = new Map([...seen].map(([key, id]) => [key, id]));
const ruleEntries = new Map();
let current = null;
for (const line of collisionsText.split("\n")) {
  const h = /^## (C\d+)\./.exec(line);
  if (h) {
    current = h[1];
    ruleEntries.set(current, new Set());
    continue;
  }
  if (/^## /.test(line)) current = null;
  if (!current || !/^(Accept|Reject|Trap): /.test(line.trim()) && !/^`[art]\d/.test(line.trim())) continue;
  for (const r of line.matchAll(/(?<!retired:)`([art])(\d{2,})`(?:–`([art])(\d{2,})`)?/g)) {
    const lo = Number(r[2]);
    const hi = r[4] ? Number(r[4]) : lo;
    for (let n = lo; n <= hi; n++) ruleEntries.get(current).add(`${r[1]}${String(n).padStart(r[2].length, "0")}`);
  }
}
for (const [rule, entries] of ruleEntries) {
  for (const key of entries) {
    if (!shortIds.has(key)) fail(`collisions.md ${rule}`, `names ${key}, which does not exist`);
  }
}
for (const e of [...accept, ...reject, ...trap]) {
  const m = NAME.exec(e.file);
  if (!m) continue;
  const key = `${m[1]}${m[2]}`;
  const rules = (headers.get(e.id).rule || "").split(", ").filter(Boolean);
  if (rules.length === 0) fail(e.id, "rule header is empty");
  for (const rule of rules) {
    if (!ruleEntries.has(rule)) fail(e.id, `cites ${rule}, which collisions.md does not define`);
    else if (!ruleEntries.get(rule).has(key)) fail(e.id, `cites ${rule}, but ${rule} does not name ${key}`);
  }
  for (const [rule, entries] of ruleEntries) {
    if (entries.has(key) && !rules.includes(rule)) fail(e.id, `${rule} names this entry, but its rule header does not cite ${rule}`);
  }
}

// tsc: one group per grants value (corpus.md section 4); in each, accept
// and trap entries together, then reject entries together. Each group
// gets the host declarations of its grants (abi.md section 6.3).
const hostDir = mkdtempSync(path.join(os.tmpdir(), "ulua-host-"));
const tscGroups = new Map();
for (const e of [...accept, ...trap, ...reject]) {
  const grants = headers.get(e.id).grants ?? "";
  const key = `${grants}|${e.dir === "reject" ? "reject" : "run"}`;
  if (!tscGroups.has(key)) tscGroups.set(key, { grants, entries: [] });
  tscGroups.get(key).entries.push(e);
}
for (const { grants, entries: group } of tscGroups.values()) {
  const names = grants === "" ? [] : grants.split(", ");
  const hostFiles = [];
  if (names.length > 0 && names.every((n) => FIXTURE_NAMES.includes(n))) {
    const file = path.join(hostDir, `host-${hostFiles.length}-${names.join("-")}.d.ts`);
    writeFileSync(file, declarations(names));
    hostFiles.push(file);
  }
  const { byFile, atEquality, other } = tscCodes(group.map((e) => e.path), hostFiles);
  for (const msg of other) fail("tsc", msg);
  for (const e of group) {
    const want = expectedCodes(headers.get(e.id).tsc);
    const got = [...byFile.get(path.resolve(e.path))].sort((a, b) => a - b);
    if (want === null) {
      fail(e.id, `tsc header is not 'accepts', 'exception TSnnnn, ...', or 'errors TSnnnn, ...'; measured [${got.map((c) => `TS${c}`).join(", ")}]`);
      continue;
    }
    if (/^exception /.test(headers.get(e.id).tsc) && !atEquality.get(path.resolve(e.path)))
      fail(e.id, "a tsc diagnostic is outside invariant 3: not TS2367 or TS2845 on an accepted equality, nor a diagnostic that names the type never");
    if (got.join(",") !== [...want].sort((a, b) => a - b).join(",")) {
      fail(e.id, `tsc codes [${got.map((c) => `TS${c}`).join(", ")}], header states ${headers.get(e.id).tsc}`);
    }
  }
}

// node: comparable accept entries print the golden; comparable trap
// entries print its prefix (CLAUDE.md invariant 5).
for (const e of [...accept, ...trap]) {
  if (headers.get(e.id)["js-comparable"] !== "yes") continue;
  const goldenPath = e.path.replace(/\.ts$/, ".expected");
  if (!existsSync(goldenPath)) {
    fail(e.id, "missing .expected");
    continue;
  }
  const goldenText = readFileSync(goldenPath, "utf8");
  const golden = goldenText.replace(/\n$/, "").split("\n");
  const { output, thrown } = await runNode(e);
  const outputText = output.map((line) => `${line}\n`).join("");
  if (e.dir === "accept") {
    if (thrown) fail(e.id, `node threw ${thrown}`);
    if (outputText !== goldenText) {
      fail(e.id, `node output differs:\n--- golden\n${goldenText}--- node\n${outputText}`);
    }
  } else {
    const last = golden.at(-1) || "";
    const want = `trap: ${headers.get(e.id).trap}`;
    if (last !== want) fail(e.id, `last golden line is '${last}', want '${want}'`);
    const prefix = golden.slice(0, -1);
    // Compare lines, not print calls: one print can write several lines.
    const lines = outputText === "" ? [] : outputText.replace(/\n$/, "").split("\n");
    if (lines.length < prefix.length || lines.slice(0, prefix.length).join("\n") !== prefix.join("\n")) {
      fail(e.id, `node output does not start with the golden prefix:\n${output.join("\n")}`);
    }
    if (!thrown && lines.length === prefix.length) {
      fail(e.id, "node completes with the golden prefix only; the entry never leaves its static type");
    }
  }
}

rmSync(hostDir, { recursive: true, force: true });
const total = accept.length + reject.length + trap.length;
if (failures.length) {
  for (const f of failures) console.log(`FAIL ${f}`);
  console.log(`corpus witnesses: ${failures.length} failure(s) over ${total} entries`);
  process.exit(1);
}
console.log(`corpus witnesses: ${accept.length} accept, ${reject.length} reject, ${trap.length} trap; tsc ${ts.version}, node ${process.version}`);
