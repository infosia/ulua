// A value that can be missing. In TypeScript you can use `undefined` or
// `null` for "no value". Ulua has only `null`, so there is one way to say
// "missing" and one test for it.

// A user can have an email address, or not. The type `string | null`
// says that `email` is a string or `null`.
type User = { name: string; email: string | null };

function contact(u: User): string {
  // Copy the field into a `const` first. After the test below, the
  // compiler knows that `email` is a string. It does not narrow
  // `u.email` itself, because a field can change between the test and
  // the read.
  const email = u.email;
  if (email != null) {
    return email;
  }
  return `${u.name} has no email`;
}

function firstLongWord(words: string[]): string {
  // `find` gives `null` when no element matches. In JavaScript it gives
  // `undefined`. Keep the result in a `const` and test it before you use
  // it.
  const found = words.find((w: string): boolean => w.length > 5);
  if (found == null) {
    return "none";
  }
  return found;
}

export function main(): void {
  print(contact({ name: "ada", email: "ada@example.com" }));
  print(contact({ name: "bob", email: null }));

  print(firstLongWord(["tree", "forest", "leaf"]));
  print(firstLongWord(["a", "b"]));

  const ages = new Map<string, number>();
  ages.set("ada", 36);

  // `get` also gives `null` for a missing key.
  const age = ages.get("bob");
  if (age == null) {
    print("no age for bob");
  }

  // `getOr` is a Ulua addition: it gives the value, or the fallback that
  // you pass when the key is missing. JavaScript has no such method.
  print(`${ages.getOr("ada", 0)} ${ages.getOr("bob", 0)}`);
}
