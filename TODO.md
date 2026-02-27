- Decide if we should favor in-crate or external tests

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
   `A: FromDataValue`, `R: IntoDataValue`:
   - `arg_count` returns 1 (or 0 if `A` is `()`).
   - `return_count` returns 1 (or 0 if `R` is `()`).
   - `execute` calls `A::from_dv(arg.unwrap())`, then `f(a)`, then `R::into_dv(r)`.
   For zero-arg functions use a separate impl (or a unit-tuple newtype) so the
   trait bounds stay coherent.
   `Wrap<F>` must implement `Debug` (a blanket `impl<F> Debug for Wrap<F>` printing
   the type name is sufficient).

4. **Expose from `built_in/mod.rs`**
   `pub use convert::{FromDataValue, IntoDataValue};`
   `pub use wrap::wrap;`

5. **Proof-of-concept registrations** (`src/built_in/mod.rs` → `get_interpret_builtins`)
   Register a handful of `std`-backed builtins using `wrap` to confirm the machinery
   works end-to-end, e.g.:
   - `str_len`:   `wrap(|s: String| s.len() as i64)`
   - `str_upper`: `wrap(|s: String| s.to_uppercase())`
   - `int_add`:   `wrap(|(a, b): (i64, i64)| a + b)`
   - `int_mul`:   `wrap(|(a, b): (i64, i64)| a * b)`
   Write interpreter integration tests for these in `tests/built_in.rs`.

**Note:** The `wrap` adapter only handles `DataValue` — it cannot represent
functions that need to inspect or produce `TemplateValue` variants
(`ParentStackMove`, `ParentLabel`, unrendered `Template`s). Any built-in that
operates on templates (e.g. a macro-like function that manipulates code as data)
must be implemented manually as a full `InterpretBuiltIn` to handle those cases.
