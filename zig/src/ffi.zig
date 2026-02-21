/// C interoperability layer for sid.
///
/// This file demonstrates the key advantage of rewriting in Zig: zero-boilerplate
/// C interop via @cImport / @cInclude.  No bindgen, no unsafe blocks, no separate
/// header-parsing step — the C headers are imported directly at compile time.
///
/// # Dynamic library loading
///
/// For runtime-loaded C libraries (dlopen / LoadLibrary), use std.DynLib:
///
///   var lib = try std.DynLib.open("libm.so.6");
///   defer lib.close();
///
///   // Look up a symbol by name; provide its function type.
///   const cos_fn = lib.lookup(*const fn (f64) callconv(.C) f64, "cos") orelse
///       return error.SymbolNotFound;
///
///   const result = cos_fn(1.0);
///
/// This is where runtime-dispatched sid built-in functions (e.g. those loaded
/// from a user-provided shared library) would be resolved.

const std = @import("std");

// Import the C standard math library.
// Zig translates the header declarations into Zig types automatically.
const c = @cImport({
    @cInclude("math.h");
});

/// Demo: call the C `cos` function through Zig's C interop.
///
/// In the full implementation, a dispatch table would map sid built-in names
/// such as "cos" to functions like this one.
///
/// Example mapping:
///   // sid program: `1.0 "cos" !`
///   //              ^^^^^^^^^^^   push Float(1.0), push label "cos", invoke
///   if (std.mem.eql(u8, built_in_name, "cos")) {
///       const arg = popFloat(state);          // pop Float from data stack
///       const result = callCFunction(arg);    // call C cos()
///       pushFloat(state, result);             // push result back
///   }
pub fn callCFunction(x: f64) f64 {
    return c.cos(x);
}
