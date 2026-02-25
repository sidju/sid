# SID Language Design

## Syntax

SID uses reverse Polish notation (RPN). Values are pushed onto a stack; functions
consume values from the top of the stack and push their results back.

### Literals

| Kind | Example | Notes |
|------|---------|-------|
| bool | `true` `false` | |
| int | `-5` `59` | Optional leading `-`, then ASCII digits |
| float | `.78` `-50.93` | Optional leading `-`, digits, at least one `.` |
| char | `'a'` `'👮‍♀️'` | Enclosed by `'`; contents must be one unicode grapheme cluster |
| string | `"ghoul"` | Enclosed by `"`; contents are unicode graphemes; backing store is bytes |
| label | `foo` `my_thing` | Bare identifier; see *Label resolution* below |
| list | `[1, "two", '3']` | Enclosed by `[]`; element types need not match |
| set | `{1, "two", 3}` | Enclosed by `{}`; no `:` at the first parsing level |
| struct | `{one: "one", two: 2.0}` | Enclosed by `{}`; `:` at the first parsing level |
| substack | `(16 16 mul sqrt)` | Enclosed by `()`; a value that can be invoked with `!` |
| script | `<"Hi" print "there" print>` | Enclosed by `<>`; like substack but sequential execution guaranteed |

### Comments

`#` begins a comment; the rest of the line is ignored.

### Invoke (`!`)

`!` pops the top of the stack and executes it as a substack or script.

### Stack substitution (`$n` / `$name`)

Inside a template literal, `$n` splices in the *n*th value from the parent stack
at render time (the value is moved, not copied). `$name` copies a label from the
enclosing scope.

```
a b ($2 ($1))   →   b (a ($1))
```

Substitution is only interpreted one level deep; inner templates keep their
`$` tokens until they are themselves invoked.

### Examples

**Declare a value to scope** (assuming `def` takes `{name: label, value: Any}`):

```
approx_pi 3 def!
```

**A match case** (assuming `match` takes `{value: Any, cases: [...]}`):

```
"Yes" [
  { case: {"yes","Yes","y","Y"}, action: (true) },
  { case: Any, action: <"That's not a yes" print! false> }
] match!
```

**Declare a function** (assuming `fn` takes `{description: str, args: type, body: substack, ret: type}`):

```
print_twice
  "Prints the given message to stdout twice"
  { message: str }
  <duplicate! print! print!>
  str
fn!
def!
```

---

## Scopes

There are exactly two scopes: **global** and **local**.

### Global scope

- Accessible from everywhere in the program.
- Can only be written to from the root of a source file.
- Root-of-file evaluation is strictly sequential and blocking to avoid
  write-order races.

### Local scope

- Exists within each function; initialised with the function's arguments.
- Shadows global scope for any overlapping names.

### Writing to scope

The built-in `def` function writes to the current scope.
Re-defining a name is only allowed in a sequentially-executing scope (file
root or a script `<…>`).

---

## Execution model

### Stacks

Each thread of execution owns two stacks:

- **Data stack** — holds the application state; empty at program start.
- **Program stack** — holds the sequence of `ProgramValue`s to execute next.

### Value lifecycle

```
parse  →  TemplateValue   (may contain $n / $name substitution slots)
render →  ProgramValue    (substitution resolved against parent stack/scope)
invoke →  DataValue       (written to the data stack)
```

Templates (`Substack`, `Script`, `List`, `Set`, `Struct`) cannot be placed
directly on the data stack, but a `Substack` *containing* them can. The inner
template is rendered when its enclosing substack is invoked.

---

## Functions

All functions in SID take exactly one argument and return exactly one value.
Multiple inputs or outputs are handled by grouping them in a struct or list.

This keeps function signatures uniform and makes currying and higher-order
combinators straightforward.

---

## Types

### Values and meta-values

Any value can be used as a type — it describes the set of values it accepts.
A **meta-value** is a value being used in a type position. The term **type**
is used when referring to a parameter or slot that accepts a meta-value.
Meta-values cannot appear where a plain value is expected; the distinction is
enforced at compile time based on what each function parameter declares.

The primitive type names are pre-defined labels in global scope:

```
bool    int    float    char    str    label
```

### Container types

A container literal is a type when any of its elements is a type,
and a plain value when all of its elements are plain values.

| Literal | Elements | Result |
|---|---|---|
| `{1, 2, 3}` | values | set value |
| `{"yes", "no"}` | values | set value (usable as an enum type) |
| `{str, int}` | types | union type |
| `{x: 1, y: 2}` | values | struct value |
| `{x: float, y: float}` | types | struct type |
| `[1, 2, 3]` | values | list value |
| `int list` | type arg | list type |
| `str int map` | type args | map type (key `str`, value `int`) |

Parametric type constructors (`list`, `set`, `map`) follow RPN order: push the
type argument(s) first, then call the constructor.

### Label resolution

A bare identifier (label) resolves **lazily**, driven by the type the consuming
parameter declares:

- If the parameter expects a `label`, the label value itself is passed — no
  scope lookup is attempted.
- If the parameter expects any other type, the label is resolved against the
  current scope and the resulting value is passed. Failing to resolve is a
  compile-time error.
- If the parameter expects `Any`, the label is resolved (passing an unresolved
  label through an `Any`-typed slot is almost never the intent).

```
approx_pi 3.14159  def!   # def expects {name: label, ...} → approx_pi is the label
approx_pi 2        add!   # add expects {a: int, b: int}   → approx_pi is resolved
```

### Function types

The type of a substack (its signature) is expressed with `fn_type!`, which
takes `{args: type, ret: type}` and returns a type. This is used when a
function accepts another function as an argument:

```
{ xs: int list, f: {n: int} int fn_type! }
```

### Type aliases

Ordinary `def` stores a type under a label:

```
Point   {x: float, y: float}    def!
Answer  {"yes", "no", "maybe"}  def!   # union of three string literals
Coords  Point list               def!
```

### Restricted types (TODO)

A predicate substack paired with a base type will eventually produce a
narrowed type whose validity the compiler can check against literal arguments
at compile time. The mechanism is reserved for a future iteration.
