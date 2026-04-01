use sid::*;
/// Tests for the `do_while` built-in.
///
/// Unlike `while_do`, the body always executes at least once before the
/// condition is checked for the first time.
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

/// Body runs once even when condition is false from the start.
#[test]
fn do_while_runs_once_when_condition_false() {
    // Stack: 42. Body drops it and pushes 0. Condition always false.
    // Unlike while_do, the body still runs once → result is 0.
    let stack = run_snippet("42 (drop! 0) (false) do_while !");
    assert_eq!(stack, vec![DataValue::Int(0)]);
}

/// Loop runs exactly twice: body flips the bool, condition duplicates and checks it.
#[test]
fn do_while_two_iterations() {
    // Stack: false.
    // Iter 1: body not! → true;  cond ($1 $1)! → [true, true];  CondLoop pops true  → continue.
    // Iter 2: body not! → false; cond ($1 $1)! → [false, false]; CondLoop pops false → exit.
    let stack = run_snippet("false (not!) (($1 $1)!) do_while !");
    assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// Body passing a value to the condition: body flips a bool and leaves a copy
/// for the condition to use as the loop Bool directly.
#[test]
fn do_while_body_passes_value_to_condition() {
    // Stack: true. expected_len = 1.
    // Body: not! → false, ($1 $1)! → [false, false] (size 2 = expected_len + 1).
    // Condition: () empty — duplicate is already the Bool. CondLoop pops false → exit.
    // Final stack: [false].
    let stack = run_snippet("true (not! ($1 $1)!) () do_while !");
    assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// Condition changing the stack size panics.
#[test]
#[should_panic(expected = "condition must leave exactly one Bool")]
fn do_while_condition_wrong_size() {
    // Body is net-0 (($1 $1)! then drop). Condition pushes two values instead of one Bool.
    run_snippet("42 (($1 $1)! drop!) (($1 $1 $1)!) do_while !");
}

/// Condition returning a non-Bool panics.
#[test]
#[should_panic(expected = "condition must leave a Bool")]
fn do_while_condition_non_bool() {
    // Body is net-0. Condition leaves an Int (duplicate of 42) instead of a Bool.
    run_snippet("42 (($1 $1)! drop!) (($1 $1)!) do_while !");
}
