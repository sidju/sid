# Core concepts:

SID was originally conceived to try out two concepts:

- Automatic paralellization through dependency analysis as part of compilation.
  - Unless a function is declared to be sequential the compiler will detect
    dependencies between its invocations (mainly detected using the local scope
    and arguments) and set as much of it in parallel as possible. When this has
    been done for the whole application possibly parallel execution can be
    grouped into sequential chains of the length that best matches the
    performance cost of synchronizing between threads (configurable).
- A soft type system based on set theory (if a value is in the set it matches
  the type constraint).
  - While unions of binary representations are allowed you have to use `match`
    to get the current binary representation before operating on it.
  - Filter operations to modify the sets in ways the compiler can easily reason
    about are provided, such as forbidding a set of values or only allowing
    values above/below a given value.

It has thereafter grown to have another goal:

- Define a very simplified semi-low-level assembly, intended to offer CPUs more
  freedom to optimise the execution.

