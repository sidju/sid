# Reordering

Use a substack with `$n` substitution to rearrange stack entries. The `swap!`
built-in handles the common two-value case; templates handle everything else.

## Swap (top two)

```
swap!
```

Equivalent to `($2 $1)!`. Consumes the top two values and pushes them back
in reversed order.

## Rotate three

```
($3 $1 $2)!   # a b c → c a b
($2 $3 $1)!   # a b c → b c a
```

## Arbitrary permutation

The pattern generalises: write any permutation of `$1` through `$N` inside a
substack and invoke it. All values between the template and the deepest `$N`
are consumed in the process.

```
($4 $2 $1 $3)!   # a b c d → d b a c
```

## Caveats

- Every `$n` **consumes** all values between the template and the referenced
  position. If you need to preserve an intermediate value, clone or reorder
  it first.
- The cognitive load of tracking positional indices grows quickly with stack
  depth. When juggling more than 4–5 values, consider binding to named locals
  instead ([Named locals](NAMED-LOCALS.md)).
