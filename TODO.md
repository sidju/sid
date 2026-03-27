- Decide if we should favor in-crate or external tests
- Probably add `pointer_write`, `pointer_read` and `pointer_offset`, to enable the
  same basic pointer usage that C allows through base syntax `*(p+2)`.
- Implement `if`/`else` (can be expressed as a two-case `match`, but a dedicated
  built-in may be ergonomic).

## Documentation and examples (next up)

- Document `matches` semantics for every type in DESIGN.md
- Add Structs/Maps section to DESIGN.md (unified Map type, label-key distinction,
  dot-label access, struct matching)
- Add Match section to DESIGN.md (`{pattern: action} match !`, type dispatch,
  enum dispatch, first-match-wins)
- Rewrite `fn` example in DESIGN.md (substack + `typed_args` + `typed_rets`)
- Verify `def` example in DESIGN.md is still valid
- Update `examples/fizz-buzz.sid`
- Update `examples/structs.sid` (`&` → `$`, use real `local!`, fix unimplemented syntax)
- Update `examples/reordering.sid` (`&n` → `$n`)
- Update `examples/nurbs.sid` (match syntax and other outdated constructs)

## AND/NOT match pattern combinators (deferred)

Extend the pattern matching system with AND (matches if all sub-patterns match)
and NOT (matches if inner pattern does not match) combinators. Syntax TBD —
design should follow the built-in constructor idiom used elsewhere.


## `&n` back-reference operator

A non-destructive stack access analogous to `$n` (which moves a value). `&n`
would copy a value from the stack without consuming it, enabling struct field
reading and other patterns where the original must be preserved. The `&`
character is reserved for this purpose.

## `describe` annotation built-in

A built-in to attach a human-readable description string to any value.
Descriptions would be dropped in release/compiled builds but remain visible in
the interpreter and debugger. Useful for self-documenting functions and types.

## `typed_rets` struct auto-unpacking (future)

Allow `typed_rets` to accept a struct type in addition to a list of types.
When a struct type is given, return values would be automatically packaged into
a named struct on return, mirroring the auto-packing behaviour of `typed_args`.

## Built-in function wrapper (`src/built_in/`)

Implement a zero-boilerplate `wrap` adapter so any compatible Rust `Fn` can be
registered as an `InterpretBuiltIn` without writing a manual wrapper per function.
See DESIGN.md § "Built-in function wrapper" for the rationale and calling convention.

Steps:

1. **`FromDataValue` trait** (`src/built_in/convert.rs`)
   Define `trait FromDataValue: Sized { fn from_dv(v: DataValue) -> anyhow::Result<Self>; }`.
   Implement for each primitive that maps directly to a `DataValue` variant:
   `bool`, `i64` (`Int`), `f64` (`Float`), `String` (`Str`), `char`-as-`String` (`Char`).
   Implement for Rust tuples `(A,)`, `(A, B)`, `(A, B, C)` … up to a reasonable arity
   by destructuring `DataValue::List` positionally, erroring on length/type mismatch.

2. **`IntoDataValue` trait** (`src/built_in/convert.rs`)
   Define `trait IntoDataValue { fn into_dv(self) -> Option<DataValue>; }`.
   Implement for the same primitives (wrapping in the matching `DataValue` variant).
   Implement for `()` returning `None` (zero-return functions).
   Implement for tuples by constructing a `DataValue::List`.

3. **`Wrap<F>` struct and `wrap` constructor** (`src/built_in/wrap.rs`)
   Define `struct Wrap<F>(F)` and `pub fn wrap<F>(f: F) -> Wrap<F>`.
   Implement `InterpretBuiltIn for Wrap<F>` where `F: Fn(A) -> R`,
   `A: FromDataValue`, `R: IntoDataValue`.
   `Wrap<F>` must implement `Debug`.

4. **Expose from `built_in/mod.rs`**
   `pub use convert::{FromDataValue, IntoDataValue};`
   `pub use wrap::wrap;`

5. **Proof-of-concept registrations** (`src/built_in/mod.rs` → `get_interpret_builtins`)
   Register a handful of `std`-backed builtins using `wrap` to confirm the machinery
   works end-to-end, e.g. `str_len`, `str_upper`, `int_add`, `int_mul`.
   Write interpreter integration tests in `tests/built_in.rs`.
