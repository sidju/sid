# Typed Functions

Using `typed_args` to get automatic label resolution and named local
bindings - the most ergonomic way to write functions in SID.

## What `typed_args` does automatically

1. **Validates argument types** at call time
2. **Resolves label arguments** — no manual `get!` needed
3. **Binds each argument into the callee's local scope** under its declared name

## Before — untyped substack with manual stack juggling

```
# Caller must manage positions and extract fields manually
# Accepts two $types.int as input
(
  clone!
  multiply!
  swap!
  clone!
  multiply!
  add!
  sqrt!
)!
```

The caller is responsible for:
- Using x and y from the top of the data stack
- Keeping values in the right stack positions
- Cleaning up intermediate values
- Not mixing up which one is x and which one is y
  (though equivalent in this case)

## After — typed substack with named locals

```
{x: $types.int, y: $types.int}
# Inside: x and y are bound as locals, no stack tracking needed
(
  x x multiply!
  y y multiply!
  add!
  sqrt!
)
typed_args!
```

The caller just puts the values on the stack, same as before (though they
have the option of putting them in a struct to clearly indicate which is
which). The type annotation tells the runtime to resolve the struct,
extract `x` and `y` by name, bind them as locals, and execute the body
with that local scope.

Since the ordering of the argument struct fields defines the order of
arguments expected this establishes a stable interface, allowing internal
changes in the function to not affect the function signature.

## Full example — a distance function

```
distance {
  x: int,
  y: int
} (
  x x multiply!
  y y multiply!
  add!
  sqrt!
) typed_args@! global@!

3 4 distance !   # → 5
```

The caller passes exactly the same values as otherwise:
1. The arguments are validated against `{x: $types.int, y: $types.int}`
  - A matching struct is a valid way to provide all arguments very
    explicitly.
  - The right types in the right order on the stack are auto-converted
    into a struct matching that format.
2. `x` is extracted and bound to local `x`
3. `y` is extracted and bound to local `y`
4. The body runs with `x` and `y` available as named locals

