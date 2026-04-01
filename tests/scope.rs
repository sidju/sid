use sid::*;
/// Integration tests for substack scope isolation, `local!` / `load_local!`,
/// and lazy label resolution at typed argument sites.
///
/// Verifies:
/// - Each substack gets a fresh local scope (bindings don't leak to the parent).
/// - Nested substacks each get their own scope.
/// - Typed-args substacks cannot read below their declared args (StackBlock).
/// - `local!` writes to the current substack's scope, not the caller's.
/// - `load_local!` unpacks a struct into the current scope.
use std::collections::HashMap;

fn run_snippet(source: &str) -> Vec<DataValue> {
    let parsed = parse_str(source).expect("parse error");
    let mut global_scope = default_scope();
    let comptime_builtins = get_comptime_builtins();
    let after_comptime =
        comptime_pass(parsed.0, &comptime_builtins, &mut global_scope).expect("comptime error");
    let rendered: DataValue = {
        let mut gs = GlobalState::new(&mut global_scope);
        render_template(
            Template::substack((after_comptime, 0)),
            &mut vec![],
            &HashMap::new(),
            &mut gs,
            &comptime_builtins,
        )
    };
    let instructions: Vec<TemplateValue> = vec![TemplateValue::from(rendered)];
    let builtins = get_interpret_builtins();
    let mut global_scope_for_run = global_scope;
    let global_state = GlobalState::new(&mut global_scope_for_run);
    let mut exe_state = ExeState {
        program_stack: vec![ProgramValue::Invoke],
        data_stack: instructions,
        local_scope: HashMap::new(),
        scope_stack: Vec::new(),
        global_state,
        builtins: &builtins,
    };
    while !exe_state.program_stack.is_empty() {
        interpret_one(
            &mut exe_state.data_stack,
            &mut exe_state.program_stack,
            &mut exe_state.local_scope,
            &mut exe_state.scope_stack,
            &mut exe_state.global_state,
            &builtins,
        );
    }
    exe_state
        .data_stack
        .into_iter()
        .filter_map(|tv| match tv {
            TemplateValue::Literal(ProgramValue::Data(v)) => Some(v),
            _ => None,
        })
        .collect()
}

fn run_and_check_outer_scope(source: &str) -> HashMap<String, DataValue> {
    let parsed = parse_str(source).expect("parse error");
    let mut global_scope = default_scope();
    let comptime_builtins = get_comptime_builtins();
    let after_comptime =
        comptime_pass(parsed.0, &comptime_builtins, &mut global_scope).expect("comptime error");
    let rendered: DataValue = {
        let mut gs = GlobalState::new(&mut global_scope);
        render_template(
            Template::substack((after_comptime, 0)),
            &mut vec![],
            &HashMap::new(),
            &mut gs,
            &comptime_builtins,
        )
    };
    let instructions: Vec<TemplateValue> = vec![TemplateValue::from(rendered)];
    let builtins = get_interpret_builtins();
    let mut global_scope_for_run = global_scope;
    let global_state = GlobalState::new(&mut global_scope_for_run);
    let mut exe_state = ExeState {
        program_stack: vec![ProgramValue::Invoke],
        data_stack: instructions,
        local_scope: HashMap::new(),
        scope_stack: Vec::new(),
        global_state,
        builtins: &builtins,
    };
    while !exe_state.program_stack.is_empty() {
        interpret_one(
            &mut exe_state.data_stack,
            &mut exe_state.program_stack,
            &mut exe_state.local_scope,
            &mut exe_state.scope_stack,
            &mut exe_state.global_state,
            &builtins,
        );
    }
    exe_state.local_scope
}

// ── local! writes to the innermost scope ─────────────────────────────────────

#[test]
fn local_binding_readable_via_nested_template() {
    // `x 42 local!` binds x=42 in the current scope (name pushed first, value on top).
    // `($x)` is a nested template rendered at runtime from that scope, so $x=42.
    let result = run_snippet("(x 42 local! ($x) !) !");
    assert_eq!(result, vec![DataValue::Int(42)]);
}

#[test]
fn local_binding_does_not_leak_to_caller() {
    // The inner substack binds x; the outer local_scope must not contain it
    // after the call completes and PopScope fires.
    let outer_scope = run_and_check_outer_scope("(x 42 local!) !");
    assert!(
        !outer_scope.contains_key("x"),
        "inner local! binding leaked to outer scope"
    );
}

#[test]
fn nested_substacks_have_independent_scopes() {
    // Outer binds x=1; inner independently binds x=2.
    // Each reads its own x via a nested template.
    let result = run_snippet("(x 1 local! (x 2 local! ($x) !) ! ($x) !) !");
    // Inner executes first, pushing 2, then outer pushes 1.
    assert_eq!(result, vec![DataValue::Int(2), DataValue::Int(1)]);
}

// ── load_local! unpacks a struct into the current scope ──────────────────────

#[test]
fn load_local_unpacks_struct_fields() {
    // {greeting: "hello"} is a struct literal (all-label keys → Struct).
    // load_local! unpacks it into local scope; ($greeting) reads it back.
    let result = run_snippet(r#"( {greeting: "hello"} load_local! ($greeting) ! ) !"#);
    assert_eq!(
        result,
        vec![DataValue::Str(std::ffi::CString::new("hello").unwrap())]
    );
}

// ── lazy label resolution at typed arg sites ─────────────────────────────────

/// A bare label whose name is in scope is automatically resolved when the
/// expected arg type does not accept labels (e.g. types.int).
#[test]
fn bare_label_resolved_at_typed_arg_site() {
    // Bind x=42 in local scope, then push the bare label `x`.
    // typed_args expects an int: label doesn't match, so it's resolved to 42.
    // Body re-pushes n via a nested template.
    let result = run_snippet("(x 42 local! x {n: $types.int} (($n) !) typed_args ! !) !");
    assert_eq!(result, vec![DataValue::Int(42)]);
}

/// A bare label is NOT resolved when the expected arg type is types.label —
/// the label value itself is the intended argument.
#[test]
fn bare_label_kept_when_label_type_expected() {
    // x is in scope (=42), but the arg type is types.label so the label is kept as-is.
    // Body re-pushes n (which is the label `x`) via a nested template.
    let result = run_snippet("(x 42 local! x {n: $types.label} (($n) !) typed_args ! !) !");
    assert_eq!(result, vec![DataValue::Label("x".to_owned())]);
}

#[test]
fn typed_substack_caller_stack_preserved_below_args() {
    // Stack: 99 (deep), 42 (the int arg consumed by PushScope into local scope as n).
    // Body is empty — arg is in local scope, not on the stack.
    // StackBlock is cleaned up; 99 must survive untouched below it.
    let result = run_snippet("99 42 {n: $types.int} () typed_args ! !");
    assert_eq!(result, vec![DataValue::Int(99)]);
}

#[test]
fn stackblock_removed_after_typed_substack_completes() {
    // A scope function with no ret — the StackBlock must be cleaned up so
    // subsequent operations see the correct stack depth.
    let result = run_snippet("42 {n: $types.int} () typed_args ! !");
    assert_eq!(result, vec![]);
}

#[test]
fn scope_fn_args_accessible_via_sub_template() {
    // Arg consumed into local scope is accessible from a sub-template inside the body.
    // n=42 is consumed by PushScope. Body: inner template ($n) renders n from local scope.
    let result = run_snippet("42 {n: $types.int} (($n) !) typed_args ! !");
    assert_eq!(result, vec![DataValue::Int(42)]);
}
