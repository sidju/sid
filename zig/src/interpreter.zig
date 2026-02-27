/// Minimal interpreter loop for the sid language.
///
/// Mirrors the structure of src/invoke/mod.rs.

const std = @import("std");
const types = @import("types.zig");
const ffi = @import("ffi.zig");

const ProgramValue = types.ProgramValue;
const DataValue = types.DataValue;
const RealValue = types.RealValue;

// ---------------------------------------------------------------------------
// ExeState
// ---------------------------------------------------------------------------

/// Execution state, equivalent to the Rust ExeState in src/invoke/mod.rs.
///
/// In the full implementation there would be separate local_scope and
/// global_scope maps; a single `scope` is used here for simplicity.
pub const ExeState = struct {
    program_stack: std.ArrayList(ProgramValue),
    data_stack: std.ArrayList(DataValue),
    /// Variable bindings: label → RealValue
    scope: std.StringHashMap(RealValue),
    allocator: std.mem.Allocator,

    pub fn init(allocator: std.mem.Allocator) !ExeState {
        return ExeState{
            .program_stack = std.ArrayList(ProgramValue).init(allocator),
            .data_stack = std.ArrayList(DataValue).init(allocator),
            .scope = std.StringHashMap(RealValue).init(allocator),
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *ExeState) void {
        for (self.program_stack.items) |*pv| pv.deinit(self.allocator);
        self.program_stack.deinit();

        for (self.data_stack.items) |*dv| dv.deinit(self.allocator);
        self.data_stack.deinit();

        var it = self.scope.iterator();
        while (it.next()) |entry| {
            self.allocator.free(entry.key_ptr.*);
            entry.value_ptr.deinit(self.allocator);
        }
        self.scope.deinit();
    }
};

// ---------------------------------------------------------------------------
// Main interpreter loop
// ---------------------------------------------------------------------------

/// Repeatedly pop and execute each ProgramValue from the program stack.
///
/// Mirrors `interpret` / `interpret_one` in src/invoke/mod.rs.
pub fn interpret(state: *ExeState) !void {
    while (state.program_stack.items.len > 0) {
        try interpretOne(state);
    }
}

fn interpretOne(state: *ExeState) !void {
    const op = state.program_stack.pop();
    switch (op) {
        // Push a real value onto the data stack
        .Real => |rv| try state.data_stack.append(.{ .Real = rv }),

        // Push a label reference onto the data stack
        .Label => |l| try state.data_stack.append(.{ .Label = l }),

        // Invoke: pop the top of the data stack and dispatch
        .Invoke => try invoke(state),
    }
}

// ---------------------------------------------------------------------------
// Invoke dispatch
// ---------------------------------------------------------------------------

fn invoke(state: *ExeState) !void {
    if (state.data_stack.items.len == 0) {
        std.debug.print("sid: invoke on empty data stack\n", .{});
        return error.EmptyStack;
    }

    const top = state.data_stack.pop();

    switch (top) {
        .Real => |rv| switch (rv) {
            // Invoking a Substack: push its contents onto the program stack
            // so execution continues in the current context.
            .Substack => |ss| {
                // Append in reverse order so the first item is popped first.
                // Ownership of each ProgramValue transfers to program_stack;
                // only the ArrayList wrapper needs to be freed here.
                var mut_ss = ss;
                var i = mut_ss.items.len;
                while (i > 0) {
                    i -= 1;
                    try state.program_stack.append(mut_ss.items[i]);
                }
                mut_ss.deinit();
            },

            // Invoking a built-in function: look it up and call it.
            // TODO: replace the stub below with a real dispatch table once
            //       built-in functions are implemented.
            .BuiltInFunction => |name| {
                std.debug.print("sid: invoke built-in '{s}'\n", .{name});

                // --- C FFI hook-in point -----------------------------------
                // This is where a built-in like "cos" would be dispatched to
                // a C function.  Example:
                //
                //   if (std.mem.eql(u8, name, "cos")) {
                //       const arg = popFloat(state);
                //       const result = ffi.callCFunction(arg);
                //       try state.data_stack.append(.{ .Real = .{ .Float = result } });
                //   }
                // ----------------------------------------------------------
                _ = ffi.callCFunction; // keep the import live
            },

            else => {
                std.debug.print("sid: cannot invoke value of type {s}\n", .{@tagName(rv)});
                return error.InvalidInvoke;
            },
        },

        // Invoking a label: look it up in scope, then invoke the result
        .Label => |l| {
            std.debug.print("sid: invoke label '{s}'\n", .{l});
            if (state.scope.get(l)) |val| {
                // Re-push resolved value and invoke it
                try state.data_stack.append(.{ .Real = val });
                try invoke(state);
            } else {
                std.debug.print("sid: undefined label '{s}'\n", .{l});
                return error.UndefinedLabel;
            }
        },
    }
}
