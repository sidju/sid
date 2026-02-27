# sid — Zig rewrite skeleton

> **Status: work-in-progress skeleton.**  
> The Rust implementation in `src/` remains authoritative.  
> This directory is a proof-of-concept to explore a potential rewrite.

---

## Why Zig?

The primary motivation is **zero-cost, zero-boilerplate C interoperability**.

| | Rust | Zig |
|---|---|---|
| Import a C header | `bindgen` + `build.rs` + `unsafe {}` everywhere | `@cImport({ @cInclude("math.h"); })` |
| Call `cos(x)` | `unsafe { libc::cos(x) }` | `c.cos(x)` |
| Runtime dlopen | `libloading` crate | `std.DynLib` (built-in) |

`sid` needs to call into arbitrary C libraries (math, system, user-provided) at
runtime.  Zig makes this natural without additional tooling.

### C FFI example — `cos`

**Rust**
```rust
// Cargo.toml: libc = "0.2"
extern "C" { fn cos(x: f64) -> f64; }

fn call_cos(x: f64) -> f64 {
    unsafe { cos(x) }
}
```

**Zig** (see [`src/ffi.zig`](src/ffi.zig))
```zig
const c = @cImport({ @cInclude("math.h"); });

fn callCFunction(x: f64) f64 {
    return c.cos(x);
}
```

---

## Building and running

Requires **Zig 0.14** (stable, early 2026).

```sh
# From this directory (zig/)
zig build run -- path/to/script.sid

# Or build only
zig build
./zig-out/bin/sid path/to/script.sid
```

---

## Module map

| Rust module | Zig module | Notes |
|---|---|---|
| `src/types.rs` | `src/types.zig` | Tagged unions replace Rust enums |
| `src/invoke/mod.rs` | `src/interpreter.zig` | `ExeState` struct + `interpret` loop |
| `src/built_in/mod.rs` | `src/ffi.zig` + TBD | C dispatch lives in `ffi.zig` |
| `src/parse/` | TBD | Parser not yet implemented |
| `src/render/` | TBD | Renderer not yet implemented |
| `src/bin/sid.rs` | `src/main.zig` | Entry point: read file → `run()` |

---

## What is not yet implemented

- **Parser** (`src/parse/` → `src/parse.zig`) — tokenise source into `ProgramValue`s
- **Renderer** — resolve template values with parent-stack captures
- **Built-in functions** — concrete implementations behind `src/ffi.zig`'s dispatch table
- **Error handling** — panics are used for now; a proper error union is planned
