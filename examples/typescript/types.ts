// Records and classes. This script models a small game: a player has a
// body that moves. It shows how you build types in Ulua without
// interfaces, inheritance, or generics.

// A record type is a `type` alias of an object type. Ulua does not
// accept `interface`; a `type` alias does the same job here.
type Vec2 = { x: number; y: number };

// A second name for the same shape. Two record types with the same
// fields and field types are the same type, so a `Position` is a `Vec2`.
type Position = { x: number; y: number };

function add(a: Vec2, b: Vec2): Vec2 {
  return { x: a.x + b.x, y: a.y + b.y };
}

// A class. Every field needs a type, and the constructor gives each
// field a value before the object is used.
class Body {
  position: Position;
  velocity: Vec2;
  constructor(x: number, y: number) {
    this.position = { x: x, y: y };
    this.velocity = { x: 1, y: 0 };
  }
  step(): void {
    // `add` takes two `Vec2` values. A `Position` fits, because the two
    // types have the same fields.
    this.position = add(this.position, this.velocity);
  }
}

// Ulua has no `extends`. In place of "a Player is a Body", write "a
// Player has a Body": keep the other object in a field and call it.
class Player {
  private score: number = 0;
  body: Body;
  constructor(name: string) {
    this.body = new Body(0, 0);
    print(`new player ${name}`);
  }
  move(): number {
    this.body.step();
    this.score += 10;
    return this.score;
  }
}

// Ulua has no generics. Write the function for the type that you need.
function largest(values: number[]): number {
  let best = values[0];
  for (const v of values) {
    if (v > best) {
      best = v;
    }
  }
  return best;
}

export function main(): void {
  const p = new Player("ada");
  p.move();
  print(`score ${p.move()}`);
  const where = p.body.position;
  print(`position ${where.x},${where.y}`);

  // A function is a value. Give the parameter and result types in the
  // arrow itself.
  const scale = (v: Vec2, k: number): Vec2 => ({ x: v.x * k, y: v.y * k });
  const far = scale(where, 3);
  print(`scaled ${far.x},${far.y}`);

  print(`largest ${largest([4, 9, 2])}`);
}
