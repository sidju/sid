/// Tests for the `do_while` built-in.
///
/// Unlike `while_do`, the body always executes at least once before the
/// condition is checked for the first time.
///
/// Each test uses the same `run_snippet` helper: parse → comptime → render →
/// interpret, returning the final data stack.
use std::collections::HashMap;
use sid::*;

fn run_snippet(source: &str) -> Vec<DataValue> {
  let parsed = parse_str(source).expect("parse error");
  let mut global_scope = default_scope();
  let comptime_builtins = get_comptime_builtins();
  let after_comptime = comptime_pass(parsed.0, &comptime_builtins, &mut global_scope)
    .expect("comptime error");
  let rendered = render_template(
    Template::substack((after_comptime, 0)),
    &mut vec![],
    &HashMap::new(),
    &global_scope,
    &comptime_builtins,
  );
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
    interpret_one(&mut exe_state, &builtins);
  }
  exe_state.data_stack.into_iter().filter_map(|tv| {
    if let TemplateValue::Literal(ProgramValue::Data(v)) = tv { Some(v) } else { None }
  }).collect()
}

/// Body runs once even when condition is false from the start.
#[test]
fn do_while_runs_once_when_condition_false() {
  // Stack: 42. Body drops it and pushes 0. Condition always false.
  // Unlike while_do, the body still runs once → result is 0.
  let stack = run_snippet("42 (drop! 0) (false) do_while !");
  assert_eq!(stack, vec![DataValue::Int(0)]);
}

/// Loop runs exactly twice: body flips the bool, condition clones and checks it.
#[test]
fn do_while_two_iterations() {
  // Stack: false.
  // Iter 1: body not! → true;  cond clone → [true, true];  CondLoop pops true  → continue.
  // Iter 2: body not! → false; cond clone → [false, false]; CondLoop pops false → exit.
  let stack = run_snippet("false (not!) (clone!) do_while !");
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// Body pushing an extra item panics with a body-specific message.
#[test]
#[should_panic(expected = "loop body must leave the stack unchanged")]
fn do_while_body_wrong_size() {
  // Body pushes an extra clone without removing it → net +1, caught before condition runs.
  run_snippet("42 (clone!) (false) do_while !");
}

/// Condition changing the stack size panics.
#[test]
#[should_panic(expected = "condition must leave exactly one Bool")]
fn do_while_condition_wrong_size() {
  // Body is net-0 (clone then drop). Condition pushes two values instead of one Bool.
  run_snippet("42 (clone! drop!) (clone! clone!) do_while !");
}

/// Condition returning a non-Bool panics.
#[test]
#[should_panic(expected = "condition must leave a Bool")]
fn do_while_condition_non_bool() {
  // Body is net-0. Condition leaves an Int (clone of 42) instead of a Bool.
  run_snippet("42 (clone! drop!) (clone!) do_while !");
}
