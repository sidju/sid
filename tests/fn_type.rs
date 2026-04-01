use sid::*;
/// Integration tests for the `fn`, `typed_args`, `typed_rets`, `untyped_args`,
/// and `untyped_rets` built-ins — exercised through the full parse → comptime
/// → render → interpret pipeline.
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

// ── fn ────────────────────────────────────────────────────────────────────────

#[test]
fn fn_pushes_unconstrained_fn_type() {
    let stack = run_snippet("fn !");
    assert_eq!(
        stack,
        vec![DataValue::Type(SidType::Fn {
            args: None,
            ret: None
        })]
    );
}

// ── typed_args ────────────────────────────────────────────────────────────────

#[test]
fn typed_args_sets_args_on_substack() {
    // Struct map deepest-first: n is the single (bottom) arg.
    let stack = run_snippet("{n: $types.int} (42) typed_args !");
    assert_eq!(
        stack,
        vec![DataValue::Substack {
            body: vec![ProgramValue::Data(DataValue::Int(42))],
            args: Some(vec![("n".to_owned(), SidType::Int)]),
            ret: None,
        }]
    );
}

#[test]
fn typed_args_sets_args_on_fn_type() {
    // Fn type stores type-only (no names at the type level).
    let stack = run_snippet("{n: $types.int} fn ! typed_args !");
    assert_eq!(
        stack,
        vec![DataValue::Type(SidType::Fn {
            args: Some(vec![SidType::Int]),
            ret: None,
        })]
    );
}

#[test]
fn typed_args_error_on_non_map() {
    // Plain lists are rejected.
    let result = std::panic::catch_unwind(|| run_snippet("{} (42) typed_args !"));
    assert!(result.is_err());
}

// ── typed_rets ────────────────────────────────────────────────────────────────

#[test]
fn typed_rets_sets_ret_on_substack() {
    let stack = run_snippet("[$types.int] (42) typed_rets !");
    assert_eq!(
        stack,
        vec![DataValue::Substack {
            body: vec![ProgramValue::Data(DataValue::Int(42))],
            args: None,
            ret: Some(vec![SidType::Int]),
        }]
    );
}

#[test]
fn typed_rets_sets_ret_on_fn_type() {
    let stack = run_snippet("[$types.bool] fn ! typed_rets !");
    assert_eq!(
        stack,
        vec![DataValue::Type(SidType::Fn {
            args: None,
            ret: Some(vec![SidType::Bool]),
        })]
    );
}

// ── chaining ──────────────────────────────────────────────────────────────────

#[test]
fn typed_args_and_rets_chained_on_substack() {
    let stack = run_snippet("[$types.bool] {n: $types.int} (42) typed_args ! typed_rets !");
    assert_eq!(
        stack,
        vec![DataValue::Substack {
            body: vec![ProgramValue::Data(DataValue::Int(42))],
            args: Some(vec![("n".to_owned(), SidType::Int)]),
            ret: Some(vec![SidType::Bool]),
        }]
    );
}

#[test]
fn typed_args_and_rets_chained_on_fn_type() {
    let stack = run_snippet("[$types.bool] {n: $types.int} fn ! typed_args ! typed_rets !");
    assert_eq!(
        stack,
        vec![DataValue::Type(SidType::Fn {
            args: Some(vec![SidType::Int]),
            ret: Some(vec![SidType::Bool]),
        })]
    );
}

// ── untyped_args / untyped_rets ───────────────────────────────────────────────

#[test]
fn untyped_args_clears_args_on_substack() {
    let stack = run_snippet("{n: $types.int} (42) typed_args ! untyped_args !");
    assert_eq!(
        stack,
        vec![DataValue::Substack {
            body: vec![ProgramValue::Data(DataValue::Int(42))],
            args: None,
            ret: None,
        }]
    );
}

#[test]
fn untyped_rets_clears_ret_on_substack() {
    let stack = run_snippet("[$types.int] (42) typed_rets ! untyped_rets !");
    assert_eq!(
        stack,
        vec![DataValue::Substack {
            body: vec![ProgramValue::Data(DataValue::Int(42))],
            args: None,
            ret: None,
        }]
    );
}
// ── invocation contracts ──────────────────────────────────────────────────────

/// A scope function with typed args passes when the stack matches.
/// Args are consumed into local scope; body sees empty stack above StackBlock.
#[test]
fn typed_args_contract_passes_on_match() {
    // 42 is int (deepest, bound as a), true is bool (top, bound as b).
    // Struct written deepest-first: {a: int, b: bool}.
    // Body is empty — args have been consumed into scope.
    let stack = run_snippet("42 true {a: $types.int, b: $types.bool} () typed_args ! !");
    assert_eq!(stack, vec![]);
}

/// Argument ordering: {a: int, b: bool} means a deepest, b on top.
/// Swapping the values should fail.
#[test]
#[should_panic(expected = "args check failed")]
fn typed_args_contract_catches_wrong_order() {
    // true pushed first (deepest), 42 on top — opposite of {a: int, b: bool}.
    run_snippet("true 42 {a: $types.int, b: $types.bool} () typed_args ! !");
}

/// Args check fires before the body runs.
#[test]
#[should_panic(expected = "args check failed")]
fn typed_args_contract_fails_on_type_mismatch() {
    run_snippet("true {n: $types.int} () typed_args ! !");
}

/// Ret check passes when the body leaves the right types.
#[test]
fn typed_rets_contract_passes_on_match() {
    let stack = run_snippet("[$types.int $types.bool] (42 true) typed_rets ! !");
    assert_eq!(stack, vec![DataValue::Int(42), DataValue::Bool(true)]);
}

/// Ret ordering: [int bool] means int deepest, bool on top after body.
/// Body that returns them in the wrong order should fail.
#[test]
#[should_panic(expected = "ret check failed")]
fn typed_rets_contract_catches_wrong_order() {
    // Body leaves bool deepest and int on top — opposite of [int bool].
    run_snippet("[$types.int $types.bool] (true 42) typed_rets ! !");
}

/// Ret check fires after the body runs.
#[test]
#[should_panic(expected = "ret check failed")]
fn typed_rets_contract_fails_on_type_mismatch() {
    run_snippet("[$types.int] (true) typed_rets ! !");
}

// ── untyped_args / untyped_rets ───────────────────────────────────────────────

#[test]
fn untyped_args_clears_args_on_fn_type() {
    let stack = run_snippet("{n: $types.int} fn ! typed_args ! untyped_args !");
    assert_eq!(
        stack,
        vec![DataValue::Type(SidType::Fn {
            args: None,
            ret: None
        })]
    );
}

#[test]
fn untyped_rets_clears_ret_on_fn_type() {
    let stack = run_snippet("[$types.bool] fn ! typed_rets ! untyped_rets !");
    assert_eq!(
        stack,
        vec![DataValue::Type(SidType::Fn {
            args: None,
            ret: None
        })]
    );
}
