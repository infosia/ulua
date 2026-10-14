// Repository hygiene scan (CLAUDE.md, Privacy and repository hygiene).
// Scans every file that git tracks or does not ignore. Prints each hit
// and exits 1 if a hit exists.
// Usage: node tools/hygiene.mjs

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { userInfo } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const files = execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", "-z"], { cwd: root })
  .toString()
  .split("\0")
  .filter(Boolean);

// The patterns are split so that this file does not match itself.
const join = (...parts) => parts.join("");
const forbidden = [
  [new RegExp(join("/Us", "ers/")), "absolute macOS home path"],
  [new RegExp(join("/ho", "me/[A-Za-z_]")), "absolute Linux home path"],
  [new RegExp(join("(^|[^\\w.])/(priv", "ate/)?(tmp|var)/")), "absolute temporary path"],
  [new RegExp(join("/Vol", "umes/")), "absolute volume path"],
  [new RegExp(join("\\$\\{?HO", "ME\\b")), "home directory variable"],
  [new RegExp(join("[A-Za-z]:[\\\\/]Us", "ers[\\\\/]"), "i"), "absolute Windows home path"],
  [new RegExp(join("(^|[\\s(\"'`=])~", "/")), "home-relative path"],
  [new RegExp(join("\\bhu", "la\\b|\\bsub", "script\\b|\\bts2", "luau\\b"), "i"), "sibling or predecessor project name"],
];
const user = userInfo().username;
if (user.length >= 3) forbidden.push([new RegExp(`\\b${user}\\b`, "i"), "local user name"]);

const hits = [];
for (const file of files) {
  let text;
  try {
    text = readFileSync(path.join(root, file), "utf8");
  } catch {
    continue;
  }
  if (text.includes("\0")) continue;
  text.split("\n").forEach((line, i) => {
    for (const [re, what] of forbidden) {
      if (re.test(line)) hits.push(`${file}:${i + 1}: ${what}: ${line.trim()}`);
    }
    // A relative path that resolves outside the repository root.
    for (const m of line.matchAll(/(?:^|[^\w.])((?:\.\.[\/\\])+[^\s)'"`\]]*)/g)) {
      const resolved = path.resolve(root, path.dirname(file), m[1].replaceAll("\\", "/"));
      if (resolved !== root && !resolved.startsWith(root + path.sep)) {
        hits.push(`${file}:${i + 1}: path outside the repository: ${m[1]}`);
      }
    }
  });
}
for (const hit of hits) console.log(hit);
console.log(hits.length ? `hygiene: FAIL (${hits.length})` : "hygiene: PASS");
process.exit(hits.length ? 1 : 0);
