# Clone

Use `$n` substitution inside a substack to duplicate values. When `$n` is
evaluated it consumes all values between the template and the referenced
position, but the referenced value can be placed back into the template
multiple times.

## Clone top value

```
($1 $1)!
```

Equivalent to `clone!`. Consumes the top two stack positions (the value and
the slot below it) and pushes two copies of the value back.

Stack effect: `a b → a b b`

## Clone deeper values

```
($2 $2)!      # clone the second value  (a b c → a b c b)
($3 $3)!      # clone the third value   (a b c d → a b c d c)
```

## Clone and reorder

Reference different positions to clone and rearrange simultaneously:

```
($2 $1 $2)!   # a b c → a b c a b   (clones second value, keeps top)
($1 $2 $1)!   # a b c → a b c b a   (clones second value, reverses order)
```

## Caveats

- Every `$n` **consumes** all values between the template and the referenced
  position. Cloning a deep value drops everything above it unless you
  explicitly reference those values too.
- The `clone!` built-in only duplicates the very top value. Templates give
  you the flexibility to clone any position, but at the cost of consuming
  intermediate values.
