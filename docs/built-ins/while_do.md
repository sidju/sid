# `while_do`

**Availability:** runtime only

Repeatedly executes a body substack while a condition substack returns `true`.
The condition is checked *before* the first execution; the body may run zero
times.

Reads naturally left-to-right: **while** `cond`, **do** `body`.

## Stack effect

```
... (cond) (body)  →  ...   # condition and body popped; loop runs in place
```

Argument order: condition is below body on the stack, matching the
left-to-right reading order.

## Contracts

- **Condition:** net +1 — must leave exactly one `Bool` on top of the state,
  with all other items unchanged.
- **Body:** net 0 — must leave the stack exactly as it found it.

Both contracts are enforced at runtime. A body violation is caught by a
`StackSizeAssert` sentinel *before* the condition runs, giving a precise error
message.

## Example

```
# Count from 0 to 3
0
(clone! 3 eq! not!)
(1 ($2 $1)! drop!)
while_do !
# stack: 3
```

## Errors

- Panics `"loop body must leave the stack unchanged"` if the body changes the
  stack size.
- Panics `"loop condition must leave exactly one Bool on top"` if the condition
  changes the stack size.
- Panics `"loop condition must leave a Bool on top of the stack"` if the
  condition pushes a non-`Bool`.
