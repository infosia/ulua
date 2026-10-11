// Node implementations of the prelude names in prelude/ulua.d.ts.
// The node witness installs them before it loads a corpus program.

export function install(output) {
  globalThis.print = (text) => {
    output.push(String(text));
  };
  globalThis.Utf8 = Object.freeze({
    byteLength: (text) => Buffer.byteLength(text, "utf8"),
  });
}
