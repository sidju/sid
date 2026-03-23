# `load_scope`

**Availability:** comptime + runtime

Pops a `Struct` and inserts each of its fields into the global scope as
individual named values.

## Stack effect

```
... Struct  →  ...   # fields written to global scope as side-effect
```

## Example

```
{x: 1, y: 2} struct @! load_scope !
# x and y are now in global scope
```

## Errors

- Panics if the top value is not a `Struct`.
