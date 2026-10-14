// Docs witness: each ```ts block followed by a ```text block in the
// tutorials runs through the example runner and prints that text.
// Usage: node tools/witness/docs.mjs <runner> <doc.md>...
// The runner is examples/run/host.c, built as docs/tutorial-typescript.md says.
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from "node:fs";
import { spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";

const [runner, ...docs] = process.argv.slice(2);
const dir = mkdtempSync(path.join(os.tmpdir(), "ulua-docs-"));
let checked = 0;
const failures = [];
const files = [];
for (const doc of docs) {
  const text = readFileSync(doc, "utf8");
  const blocks = [...text.matchAll(/```(\w+)\n([\s\S]*?)```/g)].map((m) => ({ lang: m[1], body: m[2], at: m.index }));
  for (let i = 0; i + 1 < blocks.length; i++) {
    if (blocks[i].lang !== "ts" || blocks[i + 1].lang !== "text") continue;
    if (!/export (async )?function main\(/.test(blocks[i].body)) continue;
    const file = path.join(dir, `b${checked}.ts`);
    writeFileSync(file, blocks[i].body);
    files.push(file);
    const run = spawnSync(runner, [file], { encoding: "utf8" });
    const line = text.slice(0, blocks[i].at).split("\n").length;
    if (run.stdout !== blocks[i + 1].body) {
      failures.push(`${doc}:${line}: output differs\n--- doc\n${blocks[i + 1].body}--- run\n${run.stdout}${run.stderr}`);
    }
    checked++;
  }
}
// Each program type-checks under stock tsc --strict with the prelude.
const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "../..");
for (const file of files) {
  const tsc = spawnSync("node", [path.join(root, "node_modules/typescript/bin/tsc"), "--noEmit", "--strict", "--target", "es2022",
    "--module", "esnext", "--lib", "es2022", path.join(root, "prelude/ulua.d.ts"), file], { encoding: "utf8" });
  if (tsc.status !== 0) failures.push(`${file}: tsc\n${tsc.stdout}`);
}
rmSync(dir, { recursive: true, force: true });
for (const f of failures) console.log(`FAIL ${f}`);
console.log(`docs witness: ${checked} programs, ${failures.length} failure(s)`);
process.exit(failures.length ? 1 : 0);
