# `assert`

**Availability:** comptime + runtime

Pops a `Bool` and panics with an error message if it is `false`. If `true`,
execution continues with no stack change.

## Stack effect

```
... Bool  →  ...          # on true
... Bool  →  (panic)      # on false
```

## Example

```
1 1 eq ! assert !   # passes
1 2 eq ! assert !   # panics: "assertion failed"
```

## Errors

- Panics with `"assertion failed"` if the value is `false`.
- Panics if the top value is not a `Bool`.
