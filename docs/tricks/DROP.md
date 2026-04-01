# Drop

Use `$n` substitution inside a substack to discard values. When `$n` is
evaluated it **consumes** every value between the template and the referenced
position, but only the referenced value is placed back into the template.

## Drop top value

```
($2)!
```

`$2` refers to the value two positions below the template. The top value sits
between the template and `$2`, so it is consumed and discarded.

Stack effect: `a b → a`

Equivalent to `drop!`.

## Drop N values from top

```
($2)!       # drop 1 value  (a b → a)
($3)!       # drop 2 values (a b c → a)
($4)!       # drop 3 values (a b c d → a)
```

## Drop specific positions

Reference only the positions you want to keep:

```
($3 $1)!    # a b c d → a c   (keeps 3rd and 1st, drops b and d)
($3 $2)!    # a b c d → a b   (keeps 3rd and 2nd, drops c and d)
```

## Caveats

- `($1)!` is a no-op — it wraps the top value in a substack and immediately
  invokes it, leaving the stack unchanged.
- Templates can only drop values by consuming the value below them, so if there
  is only one value on the stack templates cannot drop it. For that case you can
  use the `drop!` built-in.
- `$0` would conceptually refer to the current template, but this self-reference
  isn't currently supported.
