// Luau witnesses: the corpus through the C ABI, and luau-analyze over the
// generated Luau (specs/blocks/corpus.md section 4, pipeline.md section 8).
// Usage: node tools/witness/luau.mjs
// Needs LUAU_SOURCE_DIR for the build. LUAU_ANALYZE names luau-analyze; the
// default is the binary that tools/luau-tools.sh builds.
// It also analyzes the stress modules of pipeline.md section 5.11.
// Cost: one cargo build, two ulua-corpus processes, one luau-analyze
// process. The 14 stress modules take about 20 s of the analyzer time.

import { readFileSync, readdirSync, existsSync, rmSync, mkdirSync } from "node:fs";
import { execFileSync, spawnSync } from "node:child_process";
import path from "node:path";

// The current phase and slice. Entries of this point or earlier are checked
// in full. `P2.1` is slice 1 of P2; `P2` alone sorts after every P2 slice.
const PHASE = "P4.3";
const order = (p) => {
  const m = /^P(\d)(?:\.(\d))?$/.exec(p || "P9");
  return m ? Number(m[1]) * 10 + (m[2] ? Number(m[2]) : 9) : 99;
};

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "../..");
const failures = [];
const fail = (entry, message) => failures.push(`${entry}: ${message}`);

const entries = [];
for (const dir of ["accept", "reject", "trap"]) {
  const full = path.join(root, "corpus", dir);
  for (const file of readdirSync(full).filter((f) => f.endsWith(".ts")).sort()) {
    const p = path.join(full, file);
    const header = {};
    for (const line of readFileSync(p, "utf8").split("\n")) {
      const m = /^\/\/ ([a-z-]+): (.*)$/.exec(line);
      if (!m) break;
      header[m[1]] = m[2];
    }
    entries.push({ dir, path: p, id: `${dir}/${file.slice(0, -3)}`, header });
  }
}

execFileSync("cargo", ["build", "--quiet", "-p", "ulua_corpus"], { cwd: root, stdio: "inherit" });
// CARGO_TARGET_DIR selects the build (target/sanitize for gate.sh sanitize).
const targetDir = process.env.CARGO_TARGET_DIR || path.join(root, "target");
const runner = path.join(targetDir, "debug/ulua-corpus");
const luauDir = path.join(root, "target/corpus-luau");
rmSync(luauDir, { recursive: true, force: true });
mkdirSync(luauDir, { recursive: true });
const run = spawnSync(runner, ["--emit-luau", luauDir, ...entries.map((e) => e.path)], {
  cwd: root,
  encoding: "utf8",
  maxBuffer: 1 << 28,
});
if (run.status !== 0) {
  console.error(run.stderr);
  console.error(`ulua-corpus exited with status ${run.status}`);
  process.exit(1);
}
const results = new Map();
for (const line of run.stdout.split("\n").filter(Boolean)) {
  const r = JSON.parse(line);
  results.set(r.entry, r);
}

function golden(entry) {
  const p = entry.path.replace(/\.ts$/, ".expected");
  return existsSync(p) ? readFileSync(p, "utf8") : null;
}
const text = (output) => output.map((t) => `${t}\n`).join("");

for (const e of entries) {
  const r = results.get(e.id);
  if (!r) {
    fail(e.id, "no result from ulua-corpus");
    continue;
  }
  if (r.status === "internal") {
    fail(e.id, `internal: ${r.code} ${r.message ?? ""}`);
    continue;
  }
  const current = order(e.header.phase) <= order(PHASE);
  if (e.dir === "reject") {
    if (r.status !== "rejected") fail(e.id, `status ${r.status}, want rejected`);
    else if (current) {
      const want = /^([^:]+):/.exec(e.header["expected-error"])[1];
      if (r.code !== want) fail(e.id, `code ${r.code}, want ${want}`);
    }
    continue;
  }
  const g = golden(e);
  const full =
    e.dir === "accept"
      ? r.status === "ok" && text(r.output) === g
      : r.status === "failed" &&
        r.code === `trap:${e.header.trap}` &&
        text(r.output) === g.slice(0, g.lastIndexOf("trap:"));
  if (current && !full) fail(e.id, `status ${r.status}, code ${r.code}, output ${JSON.stringify(r.output)}`);
  if (!current && full) fail(e.id, `passes in full; change its phase header to ${PHASE}`);
  if (!current && !full && !(r.status === "rejected" && r.code === "unsupported"))
    fail(e.id, `later phase: status ${r.status}, code ${r.code}, want rejected unsupported`);
}

const stressDir = path.join(root, "target/stress-luau");
rmSync(stressDir, { recursive: true, force: true });
mkdirSync(stressDir, { recursive: true });
const stress = spawnSync(runner, ["--emit-stress", stressDir], { cwd: root, encoding: "utf8" });
if (stress.status !== 0) fail("stress", `ulua-corpus --emit-stress exited with status ${stress.status}\n${stress.stderr}`);
const stressFiles = readdirSync(stressDir).map((f) => path.join(stressDir, f));
if (stressFiles.length === 0) fail("stress", "no stress modules");

const analyze = process.env.LUAU_ANALYZE || path.join(root, "target/luau-tools/luau-analyze");
const files = readdirSync(luauDir).map((f) => path.join(luauDir, f));
if (!existsSync(analyze)) fail("luau-analyze", `not found at ${path.relative(root, analyze)}; run sh tools/luau-tools.sh`);
else if (files.length > 0) {
  const a = spawnSync(analyze, ["-j1", ...files, ...stressFiles], { encoding: "utf8", maxBuffer: 1 << 28 });
  if (a.status !== 0) fail("luau-analyze", `exit ${a.status} signal ${a.signal}\n${a.stdout}${a.stderr}`);
}

const checked = entries.filter((e) => order(e.header.phase) <= order(PHASE)).length;
console.log(`luau witnesses: ${entries.length} entries, ${checked} in phase ${PHASE} or earlier, ${files.length} generated modules, ${stressFiles.length} stress modules`);
if (failures.length) {
  for (const f of failures) console.error(`FAIL ${f}`);
  process.exit(1);
}
