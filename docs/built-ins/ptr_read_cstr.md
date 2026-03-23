# `ptr_read_cstr`

**Availability:** runtime only

Reads a null-terminated C string from a raw pointer and pushes it as a `Str`.

## Stack effect

```
... Pointer  →  ... Str
```

## Example

```
# After a C call that returns a char*:
some_c_fn ! ptr_read_cstr !   # stack: Str("…")
```

## Errors

- Panics if the top value is not a `Pointer`.
- Undefined behaviour if the pointer does not point to a valid null-terminated
  string.
