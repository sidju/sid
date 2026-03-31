use sid::*;
/// Tests for the `get`, `get_local`, and `get_global` scope-lookup built-ins.
///
/// - `get`        : local → global priority, available at comptime and runtime.
/// - `get_local`  : local scope only; errors if not found.  At comptime the
///                  local scope is empty so it always errors.
/// - `get_global` : global scope only, bypassing any local shadow.
use std::collections::HashMap;

fn run_snippet(source: &str) -> Vec<DataValue> {
    let parsed = parse_str(source).expect("parse error");
    let mut global_scope = default_scope();
    let comptime_builtins = get_comptime_builtins();
    let after_comptime =
        comptime_pass(parsed.0, &comptime_builtins, &mut global_scope).expect("comptime error");
    let rendered = {
        let mut gs = GlobalState::new(&mut global_scope);
        render_template(
            Template::substack((after_comptime, 0)),
            &mut vec![],
            &HashMap::new(),
            &mut gs,
            &comptime_builtins,
        )
    };
    let instructions: Vec<TemplateValue> = rendered.into_iter().map(TemplateValue::from).collect();
    let builtins = get_interpret_builtins();
    let mut global_scope_for_run = global_scope;
    let global_state = GlobalState::new(&mut global_scope_for_run);
    let mut exe_state = ExeState {
        program_stack: vec![ProgramValue::Invoke],
        data_stack: instructions,
        local_scope: HashMap::new(),
        scope_stack: Vec::new(),
        global_state,
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
        .filter_map(|tv| {
            if let TemplateValue::Literal(ProgramValue::Data(v)) = tv {
                Some(v)
            } else {
                None
            }
        })
        .collect()
}

// ── get ───────────────────────────────────────────────────────────────────────

/// `get` at runtime resolves from global scope when no local shadow exists.
#[test]
fn get_resolves_from_global() {
    // types.int is in global scope; get should return DataValue::Type(SidType::Int).
    let stack = run_snippet("types.int get !");
    assert_eq!(stack, vec![DataValue::Type(SidType::Int)]);
}

/// `get` at comptime resolves from global scope (local is empty at comptime).
#[test]
fn get_resolves_from_global_at_comptime() {
    let stack = run_snippet("types.int get @!");
    assert_eq!(stack, vec![DataValue::Type(SidType::Int)]);
}

/// `get` prefers a local binding over the global one when both exist.
#[test]
fn get_prefers_local_over_global() {
    // Inside a substack, shadow the global `types` with an int.
    // `get` must return the local shadow (99), not the global types map.
    let stack = run_snippet("(types 99 local! types get !) !");
    assert_eq!(stack, vec![DataValue::Int(99)]);
}

// ── get_global ────────────────────────────────────────────────────────────────

/// `get_global` resolves from global scope at runtime.
#[test]
fn get_global_resolves_from_global() {
    let stack = run_snippet("types.bool get_global !");
    assert_eq!(stack, vec![DataValue::Type(SidType::Bool)]);
}

/// `get_global` at comptime is the idiomatic way to inject a global type into
/// a comptime expression (e.g. as an argument to `require @!` or `exclude @!`).
#[test]
fn get_global_at_comptime_returns_type() {
    let stack = run_snippet("types.int get_global @!");
    assert_eq!(stack, vec![DataValue::Type(SidType::Int)]);
}

/// `get_global` bypasses a local binding and returns the global value.
#[test]
fn get_global_bypasses_local_shadow() {
    // Inside a substack, shadow the global `types` with an int.
    // `get_global` must return the global types map, not the local shadow.
    // We verify by fetching types.int from it after the call — simplest is to
    // check that the result is a Type, not the int 99.
    let stack = run_snippet("(types 99 local! types get_global !) !");
    assert!(matches!(stack.as_slice(), [DataValue::Map(_)]));
}

/// `get_global` errors when the label is not in global scope.
#[test]
#[should_panic]
fn get_global_errors_on_missing() {
    run_snippet("nonexistent_label get_global !");
}

// ── get_local ─────────────────────────────────────────────────────────────────

/// `get_local` resolves a locally bound value.
#[test]
fn get_local_resolves_local_binding() {
    let stack = run_snippet("(x 42 local! x get_local !) !");
    assert_eq!(stack, vec![DataValue::Int(42)]);
}

/// `get_local` errors when the label exists only in global scope.
#[test]
#[should_panic]
fn get_local_errors_on_global_only() {
    // types.int is global, not local — get_local must error.
    run_snippet("types.int get_local !");
}

/// `get_local` at comptime always errors because comptime local scope is empty.
#[test]
#[should_panic]
fn get_local_at_comptime_always_errors() {
    run_snippet("types.int get_local @!");
}
