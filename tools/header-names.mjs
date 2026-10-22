// Header-name check (pipeline.md §2 item 7): no header of the repository
// has the file name of a standard C or C++ header, compared without case.
// Scans every header file that git tracks or does not ignore. Prints each
// hit and exits 1 if a hit exists.
// Usage: node tools/header-names.mjs
// Cost: one `git ls-files` and one existence test per file; no file is read.

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const files = execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", "-z"], { cwd: root })
  .toString()
  .split("\0")
  .filter(Boolean)
  // A deleted file stays in the index until the deletion is staged.
  .filter((file) => existsSync(path.join(root, file)));

// C17 headers, and the C++17 headers without an extension.
const c = `assert complex ctype errno fenv float inttypes iso646 limits locale math
  setjmp signal stdalign stdarg stdatomic stdbool stddef stdint stdio stdlib
  stdnoreturn string tgmath threads time uchar wchar wctype`;
const cpp = `algorithm any array atomic bitset cassert ccomplex cctype cerrno cfenv
  cfloat charconv chrono cinttypes ciso646 climits clocale cmath codecvt complex
  condition_variable csetjmp csignal cstdalign cstdarg cstdbool cstddef cstdint
  cstdio cstdlib cstring ctgmath ctime cuchar cwchar cwctype deque exception
  execution filesystem forward_list fstream functional future initializer_list
  iomanip ios iosfwd iostream istream iterator limits list locale map memory
  memory_resource mutex new numeric optional ostream queue random ratio regex
  scoped_allocator set shared_mutex sstream stack stdexcept streambuf string
  string_view strstream system_error thread tuple type_traits typeindex typeinfo
  unordered_map unordered_set utility valarray variant vector`;
const standard = new Set([
  ...c.split(/\s+/).map((name) => `${name}.h`),
  ...cpp.split(/\s+/),
]);
const header = /\.(h|hh|hpp|hxx)$/i;

const hits = files.filter((file) => {
  const name = path.basename(file).toLowerCase();
  return (header.test(name) || !name.includes(".")) && standard.has(name);
});
for (const hit of hits) console.log(`${hit}: the name of a standard header (pipeline.md §2 item 7)`);
console.log(hits.length ? `header names: FAIL (${hits.length})` : "header names: PASS");
process.exit(hits.length ? 1 : 0);
