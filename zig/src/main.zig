const std = @import("std");
const interpreter = @import("interpreter.zig");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const args = std.os.argv;

    if (args.len < 2) {
        const stderr = std.io.getStdErr().writer();
        try stderr.writeAll(
            \\sid (zig rewrite) — a stack-based RPN language interpreter
            \\
            \\Usage: sid <script.sid>
            \\
        );
        std.process.exit(1);
    }

    const path = std.mem.span(args[1]);

    const source = std.fs.cwd().readFileAlloc(allocator, path, 1024 * 1024) catch |err| {
        const stderr = std.io.getStdErr().writer();
        try stderr.print("sid: cannot read '{s}': {}\n", .{ path, err });
        std.process.exit(1);
    };
    defer allocator.free(source);

    try run(allocator, source);
}

/// Parse, render, and interpret a sid source string.
/// This mirrors the Rust `run()` in src/bin/sid.rs.
fn run(allocator: std.mem.Allocator, source: []const u8) !void {
    // TODO: call parse(source) once the parser is implemented
    // TODO: call render(parsed) once the renderer is implemented
    // For now, just hand the source to the interpreter stub so the pipeline
    // is visible end-to-end.
    _ = source;

    var state = try interpreter.ExeState.init(allocator);
    defer state.deinit();

    try interpreter.interpret(&state);
}
