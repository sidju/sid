# sid

An experimental stack-based programming language with an LLVM code-generation
backend.

SID uses **reverse Polish notation**: values are pushed onto a stack and
functions consume values from the top. Programs are built from literals,
labels, templates (`(…)` substacks, `[…]` lists, `{…}` sets/structs,
`<…>` scripts), and the invoke operator `!`.

See [`DESIGN.md`](DESIGN.md) for the full language specification.  
See [`SPECULATIONS.md`](SPECULATIONS.md) for ideas not yet being pursued.

## Building

**Without LLVM** (interpreter only):

```sh
cargo build
cargo run --bin sid -- <source-file>
```

**With LLVM backend** (requires LLVM 18 — use the Nix dev shell):

```sh
nix develop -c cargo build --features llvm
nix develop -c cargo run --bin sid-llvm -- --emit-llvm
```

## Testing

```sh
cargo test
```

