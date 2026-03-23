/// Tests for the `while_do` built-in and the `CondLoop` sentinel.
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
  );
  let instructions: Vec<TemplateValue> = rendered.into_iter().map(TemplateValue::from).collect();
  let builtins = get_interpret_builtins();
  let mut global_scope_for_run = global_scope;
  let global_state = GlobalState::new(&mut global_scope_for_run);
  let mut exe_state = ExeState {
    program_stack: vec![ProgramValue::Invoke],
    data_stack: instructions,
    local_scope: HashMap::new(),
    global_state,
  };
  while !exe_state.program_stack.is_empty() {
    interpret_one(&mut exe_state, &builtins);
  }
  exe_state.data_stack.into_iter().filter_map(|tv| {
    if let TemplateValue::Literal(ProgramValue::Data(v)) = tv { Some(v) } else { None }
  }).collect()
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
  // Stack: 0. Condition: clone top and check == 0 (true first time, false after).
  // Body: replace top with top+1 using stack reorder + drop.
  // After one iteration: stack is 1. Condition: 1 == 0 → false. Done.
  let stack = run_snippet(
    "0 \
     (clone! 0 eq!) \
     (1 ($2 $1)! drop!) \
     while_do !"
  );
  assert_eq!(stack, vec![DataValue::Int(1)]);
}

/// Body pushing an extra item panics with a body-specific message.
#[test]
#[should_panic(expected = "loop body must leave the stack unchanged")]
fn while_do_body_wrong_size() {
  // Body pushes an extra clone without removing it → net +1.
  run_snippet("42 (true) (clone!) while_do !");
}

/// Condition changing the stack size panics.
#[test]
#[should_panic(expected = "condition must leave exactly one Bool")]
fn while_do_condition_wrong_size() {
  // Condition pushes two values instead of one Bool → size check fires.
  run_snippet("42 (clone! clone!) (drop!) while_do !");
}

/// Condition returning a non-Bool panics.
#[test]
#[should_panic(expected = "condition must leave a Bool")]
fn while_do_condition_non_bool() {
  // Condition pushes a clone of the Int (not a Bool) → type check fires.
  run_snippet("42 (clone!) (drop!) while_do !");
}
