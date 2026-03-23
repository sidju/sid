# `debug_stack`

**Availability:** comptime + runtime

Prints the top `n` items of the data stack to stderr. Useful for debugging.
Does not modify the stack.

## Stack effect

```
... Int  →  ...   # pops the count, stack otherwise unchanged
```

## Example

```
1 2 3 3 debug_stack !
# prints:
# === debug_stack (top 3 of 3) ===
#   Int(3)
#   Int(2)
#   Int(1)
```

## Errors

- Panics if the top value is not a non-negative `Int`.
