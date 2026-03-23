# `drop`

**Availability:** comptime + runtime

Discards the top value from the data stack.

## Stack effect

```
... a  →  ...
```

## Example

```
42 "unused" drop !   # stack: 42
```

## Errors

- Panics if the stack is empty.
