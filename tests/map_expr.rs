use sid::*;
/// Tests for multi-token map key and value expressions.
///
/// Verifies that `{key: expr1 expr2 !}` evaluates inline at render time,
/// enabling e.g. `{result: types.str ptr !}` instead of requiring a
/// pre-bound local variable.
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

// ── Single-token values (regression: existing behaviour preserved) ────────────

#[test]
fn single_token_map_value_unchanged() {
    let stack = run_snippet("{x: 42}");
    assert_eq!(
        stack,
        vec![DataValue::Map(vec![(
            DataValue::Label("x".to_owned()),
            DataValue::Int(42)
        ),])]
    );
}

#[test]
fn single_token_map_two_entries() {
    let stack = run_snippet("{x: 1, y: 2}");
    assert_eq!(
        stack,
        vec![DataValue::Map(vec![
            (DataValue::Label("x".to_owned()), DataValue::Int(1)),
            (DataValue::Label("y".to_owned()), DataValue::Int(2)),
        ])]
    );
}

// ── Multi-token value expressions ─────────────────────────────────────────────

#[test]
fn multi_token_value_via_local_binding() {
    // Pre-bound: result_type types.str ptr ! local!
    // Then reference it: {result: $result_type}
    // Both should produce the same map value.
    let bound = run_snippet("result_type types.str ptr ! local! {result: $result_type}");
    let inline = run_snippet("{result: types.str ptr !}");
    assert_eq!(
        bound, inline,
        "inline multi-token expr should match pre-bound local"
    );
}

#[test]
fn multi_token_value_invoke_builtin() {
    // {x: types.str ptr !} should produce a map with a pointer type value
    let stack = run_snippet("{x: types.str ptr !}");
    assert_eq!(
        stack,
        vec![DataValue::Map(vec![(
            DataValue::Label("x".to_owned()),
            DataValue::Type(SidType::Pointer(Box::new(SidType::Str)))
        ),])]
    );
}

#[test]
fn multi_token_map_mixed_inline_and_literal() {
    // {a: $types.int, b: types.str ptr !} → {a: Type(Int), b: Type(Pointer(Str))}
    // $types.int is a ParentLabel resolved at render time; types.str ptr ! is an inline invoke.
    let stack = run_snippet("{a: $types.int, b: types.str ptr !}");
    assert_eq!(
        stack,
        vec![DataValue::Map(vec![
            (
                DataValue::Label("a".to_owned()),
                DataValue::Type(SidType::Int)
            ),
            (
                DataValue::Label("b".to_owned()),
                DataValue::Type(SidType::Pointer(Box::new(SidType::Str)))
            ),
        ])]
    );
}

// ── Multi-token key expressions ───────────────────────────────────────────────

#[test]
fn multi_token_key_type_expr() {
    // Keys can also be multi-token; here types.str ptr ! evaluates to a Type key.
    let stack = run_snippet("{types.str ptr !: 42}");
    assert_eq!(
        stack,
        vec![DataValue::Map(vec![(
            DataValue::Type(SidType::Pointer(Box::new(SidType::Str))),
            DataValue::Int(42)
        ),])]
    );
}
