# `eq`

**Availability:** comptime + runtime

Pops two values and pushes `true` if they are equal, `false` otherwise.
Equality is structural — two values are equal if they have the same type and
contents.

## Stack effect

```
... a b  →  ... Bool
```

`b` is popped first (top), then `a`.

## Example

```
1 1 eq !    # true
1 2 eq !    # false
"hi" "hi" eq !  # true
```

## Errors

- Panics if either argument is not a concrete `DataValue`.
