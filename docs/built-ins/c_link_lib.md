# `c_link_lib`

**Availability:** runtime only

Opens a shared library and resolves `CFuncSig` values against it, replacing
them with callable `CFunction` values. Accepts either a single `CFuncSig` or
a `List` of them. The library is opened once and cached.

## Stack effect

```
... CFuncSig            →  ... CFunction
... [CFuncSig …]        →  ... [CFunction …]
... [CFuncSig … Str]    →  ... [CFunction …]   # Str at end = explicit lib name
```

## Example

```
"/usr/include/math.h" c_load_header !
load_scope !         # puts sqrt, sin, … into global scope
sqrt c_link_lib !    # resolves sqrt against libm
```

## Errors

- Panics if the shared library cannot be opened.
- Panics if a symbol cannot be found in the library.
