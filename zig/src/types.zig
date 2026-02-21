/// Core value types for the sid interpreter.
///
/// These mirror the Rust types in src/types.rs, adapted to Zig idioms:
///   - Rust enums with data  →  Zig tagged unions
///   - Rust Vec<T>           →  std.ArrayList(T)
///   - Rust String           →  []const u8 (owned slices where needed)

const std = @import("std");

// ---------------------------------------------------------------------------
// RealValue — a concrete, fully-evaluated value
// ---------------------------------------------------------------------------

pub const RealValue = union(enum) {
    Bool: bool,
    /// Owned UTF-8 string
    Str: []const u8,
    /// A full grapheme cluster (may be multiple bytes)
    Char: []const u8,
    Int: i64,
    Float: f64,
    /// An embedded program (substack) that can be invoked
    Substack: std.ArrayList(ProgramValue),
    /// A flat list of data values
    List: std.ArrayList(DataValue),
    /// Name of a built-in function
    BuiltInFunction: []const u8,

    pub fn deinit(self: *RealValue, allocator: std.mem.Allocator) void {
        switch (self.*) {
            .Str => |s| allocator.free(s),
            .Char => |c| allocator.free(c),
            .BuiltInFunction => |f| allocator.free(f),
            .Substack => |*ss| {
                for (ss.items) |*pv| pv.deinit(allocator);
                ss.deinit();
            },
            .List => |*lst| {
                for (lst.items) |*dv| dv.deinit(allocator);
                lst.deinit();
            },
            else => {},
        }
    }
};

// ---------------------------------------------------------------------------
// DataValue — a value that lives on the data stack
// ---------------------------------------------------------------------------

pub const DataValue = union(enum) {
    Real: RealValue,
    /// A symbolic label (looked up in scope at invoke time)
    Label: []const u8,

    pub fn deinit(self: *DataValue, allocator: std.mem.Allocator) void {
        switch (self.*) {
            .Real => |*rv| rv.deinit(allocator),
            .Label => |l| allocator.free(l),
        }
    }
};

// ---------------------------------------------------------------------------
// ProgramValue — a value on the program (instruction) stack
// ---------------------------------------------------------------------------

pub const ProgramValue = union(enum) {
    Real: RealValue,
    /// Push a label reference onto the data stack
    Label: []const u8,
    /// Pop and invoke the top of the data stack
    Invoke,
    // TODO: Template — captures parent-stack slots; see src/types.rs Template

    pub fn deinit(self: *ProgramValue, allocator: std.mem.Allocator) void {
        switch (self.*) {
            .Real => |*rv| rv.deinit(allocator),
            .Label => |l| allocator.free(l),
            .Invoke => {},
        }
    }
};
