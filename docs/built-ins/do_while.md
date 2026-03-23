# `do_while`

**Availability:** runtime only

Executes a body substack once unconditionally, then repeats while a condition
substack returns `true`. The body always runs at least once.

Reads naturally left-to-right: **do** `body`, **while** `cond`.

## Stack effect

```
... (body) (cond)  →  ...   # body and condition popped; loop runs in place
```

Argument order: body is below condition on the stack, matching the
left-to-right reading order. This is the reverse of `while_do`.

## Contracts

Same as `while_do`:

- **Body:** net 0 — must leave the stack exactly as it found it.
- **Condition:** net +1 — must leave exactly one `Bool` on top.

## Example

```
# Flip a bool until it is false (always runs at least once)
true
(not!)
(clone!)
do_while !
# stack: false
```

## Errors

Same messages as `while_do`.

## See also

- [while_do.md](while_do.md) — checks condition first; body may not run.
