# SID — Pain Points

Ergonomic issues encountered while writing `test.sid`.

---

## Loop body cannot pass data to the condition

The `do_while` and `while_do` constructs require the stack to be identical in
size at the start and end of both the body and the condition substacks.  This
means there is no direct way to produce a value in the body and consume it in
the condition.

Consequence: `test.sid` seeds a `types.null` onto the stack before the loop
purely as a dummy value so the body can `drop!` it and replace it with the
fgets return value — even though the only reason for that value on the stack
is to satisfy the size constraint.

```
types.null             # dummy seed

(
  drop!                # discard previous fgets result
  clone!
  fgets!               # produces the real value we want the condition to see
)
(
  clone! { ... } match!
)
do_while !
```

A mechanism to explicitly pass values from the body into the condition (e.g. a
designated "loop result" slot) would eliminate both the seed and the `drop!`.

**Resolution direction:** Change the stack-size invariant so that body and
condition together must leave the stack unchanged, rather than each one
individually.  This lets the body leave a value that the condition consumes,
with no dummy seed required.

---

## Arguments are consumed, requiring defensive cloning

Function calls consume their arguments.  Any value that must survive a call —
because it is needed again in the next iteration or later — must be `clone!`d
beforehand.  In `test.sid` this happens three times before the loop even
starts: once for the FILE pointer, once for the buffer size, and once for the
buffer pointer.

```
fopen!
clone!          # keep a copy for fclose later

4096 clone!     # malloc eats the size
malloc! types.str ptr_cast! clone!  # fgets eats the buffer
```

A non-destructive argument mode, or a `&n` copy-from-stack operator (already
noted in TODO.md), would reduce this noise considerably.

**Resolution direction:** This is primarily a C FFI ergonomics issue.  Add an
optional "pointer passthrough" mode to the C FFI wrapper where pointer
in-arguments are automatically returned alongside the function's return value,
so callers don't need to pre-clone pointers they wish to keep.

---

## Stack reordering requires explicit reversal substacks

There is no concise syntax to reorder a group of stack entries.  `test.sid`
uses a four-element reversal substack to get the loop's working values into the
right order before building the argument list:

```
($4 $3 $2 $1)!   # reverse top 4 entries
```

Beyond the verbosity of writing these substacks, the deeper problem is that
it is difficult to keep track of which value is at which position.  As the
number of live values on the stack grows, mentally mapping `$1`/`$2`/`$3`/`$4`
to their actual contents becomes error-prone.  A reorder, a clone, or an
intervening call can silently shift every index, and there is no in-editor
feedback to catch the mismatch.

A dedicated stack-reorder syntax or a built-in that takes a permutation
descriptor would help with the first problem, but the positional tracking
burden is a more fundamental ergonomic cost of deeply stacked values.

**Resolution direction:** Two tracks.  First, add a debug tool that displays
the current stack contents with position labels to make the tracking burden
visible during development.  Second, investigate argument binding more deeply
— it may be that named locals (`local!`/`load_local!`) already provide enough
relief in practice, or that a gap exists worth addressing.

---
