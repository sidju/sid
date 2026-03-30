# SID — Speculations

Ideas and concepts that are not currently being pursued but may become relevant
in the future. Nothing here is guaranteed to be implemented.

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

## Restricted types

A restricted type pairs a base type with a validation predicate (a substack that
receives the value and returns `true` if it is valid):

```
(0 gt) rint!   # positive integers
```

The compiler can then prove at compile time whether a given literal satisfies the
restriction, turning runtime failures into compile-time errors.

