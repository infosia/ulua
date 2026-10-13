// Node implementations of the prelude names in prelude/ulua.d.ts, and
// of the host fixtures of specs/blocks/corpus.md section 7.
// The node witness installs them before it loads a corpus program.

const FIXTURES = {
  hostAdd: () => (a, b) => a + b,
  hostConcat: () => (a, b) => a + b,
  hostNot: () => (b) => !b,
  hostLog: (output) => (text) => {
    output.push(`log:${text}`);
  },
};

export const FIXTURE_NAMES = Object.keys(FIXTURES);

// grants: the fixture names of the entry's `grants` header.
export function install(output, grants = []) {
  globalThis.print = (text) => {
    output.push(String(text));
  };
  globalThis.Utf8 = Object.freeze({
    byteLength: (text) => Buffer.byteLength(text, "utf8"),
  });
  for (const name of FIXTURE_NAMES) delete globalThis[name];
  for (const name of grants) globalThis[name] = FIXTURES[name](output);
}
