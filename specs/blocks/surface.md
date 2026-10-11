# Accepted surface

Status: Rev 8, 2026-10-11 (Rev 0 to Rev 4 the same day; P0 Phase Review
round 10 changed the form from prose to a table of syntax node kinds;
round 12 N5 names a parameter callee; Rev 6: the end of a function
with a result, P1 Phase Review round 4; Rev 7: `==` and `!=` follow
C7 Rev 15; Rev 8: unannotated boolean `let`, the end of a function, round 5).

This file is the closed list of the forms that the compiler accepts.
The list is a table of syntax node kinds, with the names of the ESTree
and `oxc` ASTs. **A node kind that this file does not list is
rejected, in every position** (collisions.md C11). A listed node kind is
accepted only in the positions and with the children and types that its
row states. `collisions.md` states why a listed form agrees with
JavaScript, or names its divergence. A form joins this list only with a contract change and
corpus entries.

Notation: "expr" is any listed expression node; "T" is a listed type.
Every type is the checked type of collisions.md C1, with the narrowing of
C16 only, not the `tsc` type.

## 1. Module and declarations

| Node kind | Accepted form |
|---|---|
| `Program` | A module. Its statements are the declarations of this table. It exports `function main(): void`. |
| `ExportNamedDeclaration` | Only `export` before a module-level `FunctionDeclaration`. No specifiers, no `export default`. |
| `FunctionDeclaration` | Not `async`, not a generator. A `BindingIdentifier` name. Each parameter is a `FormalParameter` of §4. A result type annotation is required. |
| `VariableDeclaration` | `let` or `const`, one or more `VariableDeclarator`s. |
| `VariableDeclarator` | The id is a `BindingIdentifier` (no binding pattern). An initializer is required. A type annotation is optional; without it the type is the checked type of the initializer, which must not be `null` alone or an empty array literal. A `let` without an annotation whose initializer has type `boolean` is accepted only if the initializer is a boolean literal or a comparison (`<`, `>`, `<=`, `>=`, an equality). `tsc` gives a narrowed boolean reference a literal type, and a later assignment then gives TS2322. |
| `TSTypeAliasDeclaration` | Module level only. No type parameters. The type is a `TSTypeLiteral` of §5. |
| `ClassDeclaration` | Module level only. No `extends`, `implements`, type parameters, or decorators. Members: `PropertyDefinition` and `MethodDefinition` of this table. |
| `PropertyDefinition` | An identifier key, not computed, not `static`, not `declare`, not `readonly`, not `#private`. A type annotation is required. An initializer is optional (C13). Modifier: `private`. |
| `MethodDefinition` | Kind `constructor` (at most one) or `method`. An identifier key, not computed, not `static`, not `get` or `set`, not `#private`. Parameters as for `FunctionDeclaration`. A method has a result type annotation. Modifier: `private`. |

## 2. Statements

| Node kind | Accepted form |
|---|---|
| `VariableDeclaration`, `FunctionDeclaration` | In any block. |
| `ExpressionStatement` | The expression is an `AssignmentExpression`, an `UpdateExpression`, a `CallExpression`, a `NewExpression`, or a `ChainExpression` whose last step is a call (§3). |
| `BlockStatement` | A list of statements. |
| `IfStatement` | A `boolean` test. An optional `else`. |
| `WhileStatement` | A `boolean` test. |
| `ForStatement` | The init is a `let` `VariableDeclaration` or absent. The test is `boolean` or absent. The update is an `AssignmentExpression` or an `UpdateExpression`, or absent. No function or arrow node anywhere in the head (C9.3). |
| `ForOfStatement` | The left side is a `const` declaration of one `BindingIdentifier`. The right side has an array type. Not `await`. |
| `BreakStatement`, `ContinueStatement` | No label. |
| `ReturnStatement` | With an expr, or empty in a function with a `void` result. |

The body of a function with a result other than `void` cannot
complete. A statement list cannot complete if one of its statements
cannot complete. A statement cannot complete if it is a `return`; a
block whose list cannot complete; an `if` with an `else` whose two
branches cannot complete; or a loop whose test is the literal `true`
with no parentheses or is absent, and whose body holds no `break` of
that loop. `tsc` gives TS2366 for a parenthesized `(true)` test.

## 3. Expressions

Each row states its operand types. An operand of any other type is
rejected, also where `tsc` accepts it.

| Node kind | Accepted form |
|---|---|
| `NumericLiteral` | A number. No BigInt suffix `n`. |
| `StringLiteral` | Decodes to Unicode scalar values (C5.1). |
| `TemplateLiteral` | Not tagged. Each part decodes to Unicode scalar values. Each substitution is `number`, `string`, or `boolean` (C4.8). |
| `BooleanLiteral`, `NullLiteral` | As written. |
| `ArrayExpression` | Each element is an expr. No hole, no `SpreadElement`. |
| `ObjectExpression` | Each property is an `ObjectProperty` of kind `init` with an identifier key: not computed, not shorthand, not a method, not `get` or `set`, no `SpreadElement`. |
| `ParenthesizedExpression` | Around an expr. |
| `IdentifierReference` | A binding, a parameter, a function declaration, a class (only as the callee of `NewExpression`), `Infinity`, `NaN`, `print`, `Utf8` (only as the object of `Utf8.byteLength`), `String` (only as the callee of `String(n)`). |
| `ThisExpression` | In a method or constructor, only as the object of a `StaticMemberExpression` (C13). |
| `StaticMemberExpression` | `o.f` where `o` has an object type or a class type and `f` is a field. As the callee of a `CallExpression`, `f` can be a method of a class or a field of function type. Also `xs.length` and `xs.push` on an array, `Utf8.byteLength` as a callee, and the `string` members of C5.2 as callees. No other member. |
| `ComputedMemberExpression` | `xs[i]` where `xs` has an array type and `i` has type `number`. |
| `CallExpression` | The callee is an `IdentifierReference` of a function declaration, or of a binding or a parameter of function type, a `StaticMemberExpression` above, a `ParenthesizedExpression` around an `ArrowFunctionExpression`, or `String`. Each argument is an expr; no `SpreadElement`. The argument count equals the parameter count, less trailing parameters with a default. Not optional (`f?.()`). `xs.push(e)` only as an `ExpressionStatement`. |
| `NewExpression` | The callee is the identifier of a class declaration. Arguments as for `CallExpression`. |
| `ArrowFunctionExpression` | Not `async`. Each parameter is a `FormalParameter` of §4 without a default. An expression body, or a block body with a result type annotation. An expression body of type `void` is a call, as for an `ExpressionStatement`. |
| `UnaryExpression` | `-` on a `number`. `!` on a `boolean`. |
| `BinaryExpression` | `+`, `-`, `*`, `/`, `%`, `<`, `>`, `<=`, `>=` on two numbers. `+` on two strings. `===`, `!==`, `==`, `!=` on two operands of the same type, under the operand rules of C7; `==`, `!=` with one `NullLiteral` operand (C2). |
| `LogicalExpression` | `&&`, `\|\|` on two booleans. `a ?? b` with `a: T \| null` and `b` assignable to `T` or `T \| null`. |
| `ConditionalExpression` | A `boolean` test. Both branches have one type `T`, or one branch is `null` and the result is `T \| null`. |
| `AssignmentExpression` | Only as an `ExpressionStatement` or a `for` update; its value is never used. Operators `=`; `+=` on numbers or on two strings; `-=`, `*=`, `/=`, `%=` on numbers. The target is an `IdentifierReference` of a `let` binding or a parameter, a `StaticMemberExpression` of a field (not a method), or a `ComputedMemberExpression` of an array element. |
| `UpdateExpression` | `++`, `--`, prefix or postfix, only as an `ExpressionStatement` or a `for` update. The target is a `number` binding, parameter, or field, as for `AssignmentExpression`. |
| `ChainExpression` | Exactly one optional step and no step after it: `a?.f`, or `a?.m(args)`. `a` has type `T \| null`. Only as the left operand of `??` (`a?.f`), or as an `ExpressionStatement` (`a?.m(args)`). |

Every value flow uses the assignability of collisions.md C1: an
argument to its parameter, the `push` argument to the element type, an
initializer to its annotation, a returned value to the result type, a
literal entry to its field or element type, and an assigned value to
its target.

## 4. Parameters

| Node kind | Accepted form |
|---|---|
| `FormalParameter` | A `BindingIdentifier` with a type annotation. No binding pattern, no rest element, no `?`, no modifiers. A default is allowed in a `FunctionDeclaration` or a `MethodDefinition` if the type excludes `null` (C2). |

## 5. Types

| Node kind | Accepted form |
|---|---|
| `TSNumberKeyword`, `TSBooleanKeyword`, `TSStringKeyword` | As written. |
| `TSNullKeyword` | Only as a member of a `TSUnionType`. |
| `TSVoidKeyword` | Only as the result type of a function, method, arrow, or function type. |
| `TSUnionType` | Exactly two members: `T` and `null` (C2). `T` is not `null` and not a union. |
| `TSArrayType` | `T[]`. The element type does not include `null` (C6.5). |
| `TSFunctionType` | Parameters as in §4, without defaults. A result type. |
| `TSTypeReference` | The name of a type alias or a class declaration. No type arguments, no qualified name. |
| `TSTypeLiteral` | Only as the body of a `TSTypeAliasDeclaration`. Members are `TSPropertySignature`s with an identifier key and a type annotation: not optional, not computed, no index signatures, no method signatures. |
| `TSParenthesizedType` | Around a type. |

## 6. Library

| Name | Members |
|---|---|
| prelude | `print(text: string)`, `Utf8.byteLength(text: string)` |
| global | `Infinity`, `NaN`, `String(n)` with `n: number` |
| `string` | the members of collisions.md C5.2 |
| `T[]` | `length` (read); `push` with one argument of type `T`, as an expression statement only |

Every other global, namespace, and member is rejected, for example
`Math`, `Number`, `Array`, `Object`, `JSON`, and every other method of
`string` and `T[]`.
