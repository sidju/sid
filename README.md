# `sid`

`sid` is a modern stack-based language: Forth-like in its low-level control and
FFI friendliness, Joy-like in its functional composition via
quotations/closures, with a simple flexible type system and built-in lightweigth
data structures.

See [`docs/README.md`](docs/README.md) for an introduction into the language
and its syntax.
  
See [`SPECULATIONS.md`](SPECULATIONS.md) for wilder ideas for where the language
could develop.


## Building

**Without LLVM** (interpreter only):

```sh
cargo build
cargo run --bin sid -- <source-file>
```

**With LLVM backend** (requires LLVM 18 — use `nix develop`):

```sh
nix develop -c cargo build --features llvm
nix develop -c cargo run --bin sid-llvm -- --emit-llvm
```

## Testing

```sh
cargo test
```
