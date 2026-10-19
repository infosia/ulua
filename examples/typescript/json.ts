// JSON. `JSON.stringify` works as in JavaScript. `JSON.parse` needs to
// know the type that you expect, and it checks the text against that
// type. In TypeScript, `JSON.parse` returns `any`, and a wrong shape
// shows up later as a strange bug. In Ulua it fails at the parse.

type Item = { name: string; count: number };
type Order = { id: number; items: Item[]; paid: boolean };

export function main(): void {
  const order: Order = { id: 7, items: [{ name: "apple", count: 3 }], paid: false };
  // The fields come out in the order of the type declaration.
  print(JSON.stringify(order));

  // The type annotation on `parsed` tells `JSON.parse` what to expect.
  // Without a target type, Ulua does not accept the call.
  const parsed: Order = JSON.parse('{"id":8,"items":[],"paid":true}');
  print(`order ${parsed.id}, ${parsed.items.length} items, paid ${parsed.paid}`);

  // A field that the type does not declare is dropped. node keeps it and
  // prints `{"id":9,"note":"rush","items":[],"paid":true}`.
  const withNote: Order = JSON.parse('{"id":9,"note":"rush","items":[],"paid":true}');
  print(JSON.stringify(withNote));

  // Text with the wrong shape throws `TypeError`. Here `id` is a string,
  // and the type says `number`. node accepts the text and gives a string
  // `id`.
  try {
    const bad: Order = JSON.parse('{"id":"ten","items":[],"paid":true}');
    print(`${bad.id}`);
  } catch (e) {
    if (e instanceof TypeError) {
      print("the text does not match Order");
    }
  }

  // Text that is not JSON throws `SyntaxError`, as in JavaScript.
  try {
    const broken: Item = JSON.parse("{");
    print(broken.name);
  } catch (e) {
    if (e instanceof SyntaxError) {
      print("the text is not JSON");
    }
  }
}
