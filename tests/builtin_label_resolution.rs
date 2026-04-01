use sid::*;
/// Tests that built-ins resolve a bare label to the value it points to in scope
/// when the direct value would have been accepted but a `Label` was received
/// instead.
///
/// Before the fix, built-ins such as `assert`, `not`, `while_do`, `do_while`,
/// `load_local`, and `load_scope` bailed with a type-mismatch error whenever
/// a `Label` appeared where a Bool / Substack / Map / etc. was expected.
/// After the fix they fall back to resolving the label via scope and use the
/// resolved value transparently.
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

// ── assert ────────────────────────────────────────────────────────────────────

/// A label pointing to `true` should be accepted by `assert` without error.
#[test]
fn assert_resolves_true_label() {
    // Binds `flag = true` in local scope, then pushes the bare label `flag`.
    // `assert` must resolve `flag` → true and succeed (no panic, empty stack).
    let stack = run_snippet("(flag true local! flag assert !) !");
    assert_eq!(stack, vec![]);
}

/// A label pointing to `false` should trigger the assertion-failed error.
#[test]
#[should_panic(expected = "assertion failed")]
fn assert_resolves_false_label() {
    run_snippet("(flag false local! flag assert !) !");
}

// ── not ───────────────────────────────────────────────────────────────────────

/// A label pointing to `false` should be negated by `not`.
#[test]
fn not_resolves_label() {
    let stack = run_snippet("(flag false local! flag not !) !");
    assert_eq!(stack, vec![DataValue::Bool(true)]);
}

/// A label pointing to `true` should be negated to `false`.
#[test]
fn not_resolves_true_label() {
    let stack = run_snippet("(flag true local! flag not !) !");
    assert_eq!(stack, vec![DataValue::Bool(false)]);
}

// ── while_do ─────────────────────────────────────────────────────────────────

/// Labels pointing to the condition and body substacks are resolved by
/// `while_do`, which should then loop normally.
#[test]
fn while_do_resolves_substack_labels() {
    // cond: always false  →  body never runs, stack stays 42.
    let stack = run_snippet(
        "(cond (false) local! \
     body (drop! 0) local! \
     42 cond body while_do !) !",
    );
    assert_eq!(stack, vec![DataValue::Int(42)]);
}

/// Labels pointing to substacks work for a loop that actually runs.
#[test]
fn while_do_label_runs_loop() {
    // Counts from 0 to 1: cond checks == 0, body increments.
    let stack = run_snippet(
        "(my_cond (($1 $1)! 0 eq!) local! \
     my_body (1 ($2 $1)! drop!) local! \
     0 my_cond my_body while_do !) !",
    );
    assert_eq!(stack, vec![DataValue::Int(1)]);
}

// ── do_while ─────────────────────────────────────────────────────────────────

/// Labels pointing to body and condition substacks are resolved by `do_while`.
#[test]
fn do_while_resolves_substack_labels() {
    // body increments 0→1; cond checks duplicate==0 (false after increment) → one iteration.
    let stack = run_snippet(
        "(my_body (1 ($2 $1)! drop!) local! \
     my_cond (($1 $1)! 0 eq!) local! \
     0 my_body my_cond do_while !) !",
    );
    assert_eq!(stack, vec![DataValue::Int(1)]);
}

// ── load_local ────────────────────────────────────────────────────────────────

/// A label pointing to a Map is resolved before `load_local` unpacks it.
#[test]
fn load_local_resolves_map_label() {
    // Bind `my_map` to a struct in outer scope, then pass the bare label to
    // `load_local` inside an inner substack — it should unpack the fields.
    let stack = run_snippet(
        "(my_map {answer: 42} local! \
     my_map load_local! \
     ($answer) !) !",
    );
    assert_eq!(stack, vec![DataValue::Int(42)]);
}

// ── load_scope ────────────────────────────────────────────────────────────────

/// A label pointing to a Map is resolved before `load_scope` unpacks it.
#[test]
fn load_scope_resolves_map_label() {
    // `{answer: 99}` is stored as a Map in local scope under `exports`.
    // Passing the bare label `exports` to `load_scope` should work and make
    // `answer` available in the global scope for subsequent `$answer` resolution.
    let stack = run_snippet(
        "(exports {answer: 99} local! \
     exports load_scope! \
     ($answer) !) !",
    );
    assert_eq!(stack, vec![DataValue::Int(99)]);
}
