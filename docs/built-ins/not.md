# `not`

**Availability:** comptime + runtime

Pops a `Bool` and pushes its logical negation.

## Stack effect

```
... Bool  →  ... Bool
```

## Example

```
true not !   # false
false not !  # true
```

## Errors

- Panics if the top value is not a `Bool`.
