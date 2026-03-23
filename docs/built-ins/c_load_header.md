# `c_load_header`

**Availability:** comptime + runtime

Parses a C header file (via the system preprocessor) and returns a `Struct`
mapping function names to their `CFuncSig` values. The signatures can then be
passed to `c_link_lib` to resolve them against a shared library.

## Stack effect

```
... Str                →  ... Struct   # header path; lib name derived from filename stem
... [Str Str]          →  ... Struct   # [header_path, lib_name] — explicit lib name
```

## Example

```
"/usr/include/math.h" c_load_header !
# returns Struct of (fn_name → CFuncSig) pairs, lib_name = "math"

["/usr/include/math.h" "libm.so.6"] c_load_header !
# same, with explicit lib name
```

## Notes

- Struct definitions and typedefs are skipped; only function declarations are
  extracted.
- Variadic functions (e.g. `printf`) are included and marked as variadic.

## Errors

- Panics if the header file cannot be found or parsed.
- Panics if the argument is not a `Str` or a two-element `List` of strings.
