use sid::*;
/// Tests for the `while_do` built-in and the `CondLoop` sentinel.
///
/// Each test uses the same `run_snippet` helper: parse → comptime → render →
/// interpret, returning the final data stack.
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

/// Loop body runs zero times when condition is false from the start.
#[test]
fn while_do_zero_iterations() {
    // Stack: 42. Condition always false. Body should never run.
    let stack = run_snippet("42 (false) (drop! 0) while_do !");
    assert_eq!(stack, vec![DataValue::Int(42)]);
}

/// Loop runs exactly once: condition true on entry, false after body increments value.
#[test]
fn while_do_single_iteration() {
    // Stack: 0. Condition: duplicate top and check == 0 (true first time, false after).
    // Body: replace top with top+1 using stack reorder + drop.
    // After one iteration: stack is 1. Condition: 1 == 0 → false. Done.
    let stack = run_snippet(
        "0 \
     (($1 $1)! 0 eq!) \
     (1 ($2 $1)! drop!) \
     while_do !",
    );
    assert_eq!(stack, vec![DataValue::Int(1)]);
}

/// Initial condition can consume a setup value, with body and condition sharing
/// the resulting expected_len. Demonstrates the relaxed combined invariant.
#[test]
fn while_do_initial_cond_consumes_setup_value() {
    // Stack: [true, false] (true deeper, false on top).
    // Initial cond (drop! ($1 $1)!): drops false, duplicates true → [true, true]. Pops true → expected_len=1.
    // Body (not! ($1 $1)!): not!→false, duplicate→[false,false] (body net+1, size=expected_len+1).
    // Cond (drop! ($1 $1)!): drop false, duplicate false→[false,false]. Pops false→exit.
    // Final stack: [false].
    let stack = run_snippet("true false (drop! ($1 $1)!) (not! ($1 $1)!) while_do !");
    assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// Condition returning wrong stack size (via CondLoop on subsequent iter) panics.
#[test]
#[should_panic(expected = "condition must leave exactly one Bool")]
fn while_do_condition_wrong_size() {
    // Initial cond (true) enters loop. Body (($1 $1 $1)!) grows stack to 3.
    // Subsequent cond (true) pushes one more → size 4. CondLoop expects 2. Fires.
    run_snippet("42 (true) (($1 $1 $1)!) while_do !");
}

/// Condition returning a non-Bool panics.
#[test]
#[should_panic(expected = "condition must leave a Bool")]
fn while_do_condition_non_bool() {
    // Condition pushes a duplicate of the Int (not a Bool) → type check fires.
    run_snippet("42 (($1 $1)!) (drop!) while_do !");
}
