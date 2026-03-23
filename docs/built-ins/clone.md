# `clone`

**Availability:** comptime + runtime

Duplicates the top value on the data stack.

## Stack effect

```
... a  →  ... a a
```

## Example

```
42 clone !   # stack: 42 42
```

## Errors

- Panics if the stack is empty.
