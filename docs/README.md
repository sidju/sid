# SID Language Reference

SID is a stack-based language using Reverse Polish Notation (RPN). Values are
pushed onto a data stack; functions consume values from the top and push their
results back.

---

## Literals

| Kind     | Syntax                        | Notes |
|----------|-------------------------------|-------|
| bool     | `true` `false`                | |
| int      | `42` `-7`                     | Optional leading `-`, then ASCII digits |
| float    | `3.14` `-0.5`                 | Optional leading `-`; must contain `.` |
| char     | `'a'` `'👮'`                  | Single Unicode grapheme cluster, enclosed in `'` |
| string   | `"hello"`                     | Unicode text; backing store is bytes (C-compatible) |
| label    | `foo` `my_thing`              | Bare identifier; resolved lazily (see [Labels](#labels)) |
| list     | `[1, "two", '3']`             | Enclosed in `[]`; element types need not match; bare labels inside are stored as `Label` values — use `$name` to embed the resolved scope value instead |
| set      | `{1, 2, 3}`                   | Enclosed in `{}`; no `:` at the top parsing level; same label/`$name` distinction as list |
| map      | `{x: 1, y: 2}`                | Enclosed in `{}`; `:` at the top parsing level; keys are always label *values*; value positions share the same label/`$name` distinction |
| substack | `(16 16 mul sqrt)`            | Enclosed in `()`; a callable value, invoked with `!` |
| script   | `<"hi" print! "there" print!>`| Enclosed in `<>`; like substack but sequential execution guaranteed |

---

## Comments

`#` begins a comment; the rest of the line is ignored.

```
42 # this is a comment
```

---

## Invoke (`!`)

`!` pops the top of the stack and executes it. The value must be a substack,
script, built-in, or C function.

```
(1 2 add) !   # pushes the substack, then invokes it
clone !       # label resolves to the built-in, then invokes it
```

The shorthand `foo !` is so common it is written `foo!` throughout examples.

## Comptime Invoke (`@!`)

`@!` marks an invocation to be evaluated during the comptime pass, before
any code runs. The annotation is contagious: all `!` tokens inside a
`@!`-invoked body are also treated as comptime.

```
int list @!   # builds a List type at comptime
```

---

## Stack Substitution (`$n` / `$name`)

Inside a template literal, `$n` splices in the *n*th value from the parent
stack at render time (the value is moved, not copied). `$name` copies a label
value from the enclosing scope.

**Indexing:** `$1` is the *deepest* consumed value; `$N` is the *top*. Given
stack `[a, b]` where `b` is on top, `$1 = a` and `$2 = b`.

```
10 20 ($2 $1) !   # renders to (20 10) — swaps the two values
```

Substitution is only interpreted one level deep: inner templates keep their
`$` tokens until they are themselves invoked.

**Labels inside container literals:** A bare label inside `[…]`, `{…}` is
stored as a `Label` value — a distinct type from the resolved scope value.
Use `$name` when you want the resolved value instead:

```
[int bool]           # a list containing Label("int") and Label("bool")
[$int $bool]         # a list containing Type(Int) and Type(Bool)
{$int, $str}         # a set containing the Type values Int and Str
```

Which form is correct depends entirely on what the consuming code expects.
Built-ins like `typed_args` and `typed_rets` require `Type(…)` values, so
use `$name` there. See [built-ins/typed_args.md](built-ins/typed_args.md).

---

## Labels

A bare identifier resolves *lazily*, driven by how it is consumed:

- **In a type position** — the label itself is the value (no lookup).
- **In any other position** — looked up in local scope, then global scope.
  Failure to resolve is a runtime panic.
- **`Any`-typed slot** — the label is resolved (passing an unresolved label
  through `Any` is almost never the intent).

Pre-defined labels in global scope (type values):

| Label   | Type value         |
|---------|--------------------|
| `bool`  | `SidType::Bool`    |
| `int`   | `SidType::Int`     |
| `float` | `SidType::Float`   |
| `char`  | `SidType::Char`    |
| `str`   | `SidType::Str`     |
| `Any`   | `SidType::Any`     |

---

## Type System

Types are first-class values. A value used in a type position is a
*meta-value*; the term *type* refers to a slot that accepts one.

### Primitive types

`bool` `int` `float` `char` `str` `label`

### Container types

| Expression              | Result              |
|-------------------------|---------------------|
| `{1, 2, 3}`             | set value           |
| `{"yes", "no"}`         | set value (enum)    |
| `{str, int}`            | union type          |
| `{x: 1, y: 2}`          | map value           |
| `{x: float, y: float}`  | struct type         |
| `[1, 2, 3]`             | list value          |
| `[$int] list @!`        | `List(Int)` type    |
| `[$str $int] map @!`    | `Map(Str, Int)` type|

### Function types

`fn` pushes an unconstrained callable type. Annotate it with `typed_args`
and `typed_rets` to constrain the signature:

```
fn [$int $int] typed_args ! [$bool] typed_rets !
# → Fn { args: Some([Int, Int]), ret: Some([Bool]) }
```

A `None` dimension (not set) matches any callable — typed or untyped — on
that dimension. A `Some` dimension requires the callable to carry a matching
type annotation set via `typed_args`/`typed_rets`.

---

## Execution Pipeline

```
source text
  ↓  parse
Vec<TemplateValue>        # may contain $n / $name substitution slots
  ↓  comptime pass
Vec<TemplateValue>        # @! sites evaluated; must have concrete inputs
  ↓  render
Vec<ProgramValue>         # $n/$name resolved against parent stack/scope
  ↓  interpret
DataValue                 # written to the data stack
```

---

## Scopes

| Scope  | Accessible from | Writable from          |
|--------|-----------------|------------------------|
| Global | everywhere      | file root only         |
| Local  | inside a function | function arguments   |

Local scope shadows global for overlapping names.

---

## Further Reading

- [built-ins/](built-ins/) — one file per built-in function
