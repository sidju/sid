# SID — Speculations

Ideas and concepts that are not currently being pursued but may become relevant
in the future. Nothing here is guaranteed to be implemented.

---

## Automatic parallelisation

The compiler could detect data dependencies between function invocations
(primarily through local scope and argument analysis) and schedule independent
segments to run in parallel automatically.

Unless a function is explicitly declared sequential, the compiler would build
a dependency graph and group independent segments into parallel batches, sized
to amortise thread-synchronisation overhead (configurable).

Two possible execution models were considered:

- **Batch execution** — run a large group of independent segments until a
  synchronisation point, build the next batch, repeat.
- **Dynamic execution** — schedule segments as their dependencies are satisfied.

The long-term vision included compiling these parallel segments to run directly
on GPUs or similar data-parallel hardware, using green threads on conventional
CPUs.

---

## Set-based / validity type system

The core idea: a type is simply a *set of valid values*.

### Base types and type construction

1. A literal set `{1, 2}` is a type that accepts exactly those values (enum-like).
2. A struct type is a type accepting structs where every field is a subset of the
   corresponding field in the type-struct.
3. Set operations on existing types: `set_sub(minuend: int, subtrahend: {0})`
   gives all non-zero integers.

A single set can mix root types — `{true, "true"}` is a valid type — though
matching must be used to recover the concrete root type before operating on it.

### Meta-types

`list int` — a list of integers  
`set int` — a set of integers  
`map (key: str, val: int)` — map from string to int

Meta-types accept custom type arguments: `list (x: int, y: int)` is a list of
`{x: int, y: int}` structs.

### Restricted types

A restricted type pairs a base type with a validation predicate (a substack that
receives the value and returns `true` if it is valid):

```
(0 gt) rint!   # positive integers
```

The compiler can then prove at compile time whether a given literal satisfies the
restriction, turning runtime failures into compile-time errors.

### Validity typing

Validity typing is implicit for values and explicit for function arguments. It
only permits automatic conversions that are no-ops at the binary level and where
the source value satisfies the target type's restrictions.

Example: `divide(dividend: 4, divisor: 0)` is a compile-time type error because
`0` does not satisfy the non-zero restriction on the divisor parameter.

### Alternatives considered

- **C-compatible types** — match C's integer widths (`int8`, `uint32`, etc.)
  plus grapheme/string and collection types, dropping the set-based layer.
- **No typing** — skip the type system entirely in the short term; add it later.
  Rejected because it would require syntax changes when types are eventually
  added.

---

## Wild ideas

### Functions as match cases

Every function definition is a pattern that validates its input. If an
invocation matches no pattern an `invalid` action is triggered; the compiler
errors at compile time if that action is statically reachable.

This could eliminate explicit function-declaration syntax — a match case with a
validation predicate *is* the function definition.

### Map-keyed match

```
input [
  [{"yes", "Yes"}, (true)],
  [str, <"Taken as no" print! false>],
  [Any, <"Bad input string" print! false>],
] match!
```

The keys of a dictionary (or list of `[pattern, action]` pairs) form the match
arms. Set operations on the key types can statically prove completeness.

### Lists as tuples

Named-field structs and positional lists could share an implementation, making
lists usable as lightweight tuples without introducing a separate type.

---

## Structs, match, and reusable functions (TODO scratchpad)

The following were identified as the minimum features needed to write significant
programs:

- **Structs** — a dictionary where all keys are labels; performance deferred.
- **Match** — simplest form: a built-in that takes a dictionary; the `default`
  key is used when no other key matches. A possible sugar using `dict_get`:

  ```
  { default: ("Hello" print), formal: ("Good day" print) }
  informal dict_get !! !
  ```

- **Reusable functions** — `def` can store a substack by name; the missing piece
  is syntactic sugar for looking up and invoking entries in a dictionary without
  boilerplate.
