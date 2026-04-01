# Types

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
