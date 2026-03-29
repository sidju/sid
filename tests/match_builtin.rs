/// Tests for the `match` built-in.
///
/// Calling convention: `value {pattern: action, ...} match !`
/// Cases are a Map or Struct.  Map keys are used directly as patterns
/// (`$types.int` etc. for type dispatch); Struct keys are exact label matches.
/// First-match-wins; value is consumed; action body executes in its place.
use std::collections::HashMap;
use sid::*;

fn run_snippet(source: &str) -> Vec<DataValue> {
  let parsed = parse_str(source).expect("parse error");
  let mut global_scope = default_scope();
  let comptime_builtins = get_comptime_builtins();
  let after_comptime = comptime_pass(parsed.0, &comptime_builtins, &mut global_scope)
    .expect("comptime error");
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
    interpret_one(&mut exe_state, &builtins);
  }
  exe_state.data_stack.into_iter().filter_map(|tv| {
    if let TemplateValue::Literal(ProgramValue::Data(v)) = tv { Some(v) } else { None }
  }).collect()
}

// ── Exact literal match ───────────────────────────────────────────────────────

/// First case matches.
#[test]
fn match_exact_literal_first_case() {
  let stack = run_snippet("1 {1: (42), 2: (99)} match !");
  assert_eq!(stack, vec![DataValue::Int(42)]);
}

/// Second case matches when first doesn't.
#[test]
fn match_exact_literal_second_case() {
  let stack = run_snippet("2 {1: (42), 2: (99)} match !");
  assert_eq!(stack, vec![DataValue::Int(99)]);
}

// ── Type pattern (Map with $-resolved type keys) ──────────────────────────────

/// `$types.int` in key position resolves to the Int type and matches any integer.
#[test]
fn match_type_pattern_int() {
  let stack = run_snippet("7 {$types.int: (true), $types.bool: (false)} match !");
  assert_eq!(stack, vec![DataValue::Bool(true)]);
}

/// `$types.bool` matches a bool.
#[test]
fn match_type_pattern_bool() {
  let stack = run_snippet("true {$types.int: (1), $types.bool: (2)} match !");
  assert_eq!(stack, vec![DataValue::Int(2)]);
}

/// `$types.any` as a catch-all always matches.
#[test]
fn match_any_catch_all() {
  let stack = run_snippet(r#""hello" {$types.any: (99)} match !"#);
  assert_eq!(stack, vec![DataValue::Int(99)]);
}

// ── Enum / label dispatch (Struct with bare label keys) ───────────────────────

/// Bare label keys in `{…}` produce a Struct; each key matches the exact label.
#[test]
fn match_label_enum_dispatch() {
  let stack = run_snippet("a {a: (1), b: (2)} match !");
  assert_eq!(stack, vec![DataValue::Int(1)]);
}

// ── Cases stored in a label ───────────────────────────────────────────────────

/// Cases Map stored via `local!` is resolved at match time.
#[test]
fn match_cases_from_label() {
  let stack = run_snippet(
    "cases {1: (10), 2: (20)} local! \
     2 cases match !"
  );
  assert_eq!(stack, vec![DataValue::Int(20)]);
}

// ── Action stack effects ──────────────────────────────────────────────────────

/// Action may grow the stack.
#[test]
fn match_action_grows_stack() {
  let stack = run_snippet("42 {$types.int: (1 2)} match !");
  assert_eq!(stack, vec![DataValue::Int(1), DataValue::Int(2)]);
}

// ── Error cases ───────────────────────────────────────────────────────────────

/// No matching case panics.
#[test]
#[should_panic(expected = "match: no case matched")]
fn match_no_case_panics() {
  run_snippet("42 {$types.bool: (0)} match !");
}

/// Non-map cases argument panics.
#[test]
#[should_panic(expected = "match: cases must be a Map")]
fn match_non_map_cases_panics() {
  run_snippet("42 99 match !");
}

/// Non-substack action panics.
#[test]
#[should_panic(expected = "match: action must be a Substack or Script")]
fn match_non_substack_action_panics() {
  run_snippet("42 {42: 99} match !");
}

