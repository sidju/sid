/// Integration tests for the `fn`, `typed_args`, `typed_rets`, `untyped_args`,
/// and `untyped_rets` built-ins — exercised through the full parse → comptime
/// → render → interpret pipeline.
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

// ── fn ────────────────────────────────────────────────────────────────────────

#[test]
fn fn_pushes_unconstrained_fn_type() {
  let stack = run_snippet("fn !");
  assert_eq!(stack, vec![DataValue::Type(SidType::Fn { args: None, ret: None })]);
}

// ── typed_args ────────────────────────────────────────────────────────────────

#[test]
fn typed_args_sets_args_on_substack() {
  let stack = run_snippet("(42) [$int] typed_args !");
  assert_eq!(stack, vec![DataValue::Substack {
    body: vec![ProgramValue::Data(DataValue::Int(42))],
    args: Some(vec![SidType::Int]),
    ret:  None,
  }]);
}

#[test]
fn typed_args_sets_args_on_fn_type() {
  let stack = run_snippet("fn ! [$int] typed_args !");
  assert_eq!(stack, vec![DataValue::Type(SidType::Fn {
    args: Some(vec![SidType::Int]),
    ret:  None,
  })]);
}

#[test]
fn typed_args_error_on_non_type_list() {
  let result = std::panic::catch_unwind(|| run_snippet("(42) [1] typed_args !"));
  assert!(result.is_err());
}

// ── typed_rets ────────────────────────────────────────────────────────────────

#[test]
fn typed_rets_sets_ret_on_substack() {
  let stack = run_snippet("(42) [$int] typed_rets !");
  assert_eq!(stack, vec![DataValue::Substack {
    body: vec![ProgramValue::Data(DataValue::Int(42))],
    args: None,
    ret:  Some(vec![SidType::Int]),
  }]);
}

#[test]
fn typed_rets_sets_ret_on_fn_type() {
  let stack = run_snippet("fn ! [$bool] typed_rets !");
  assert_eq!(stack, vec![DataValue::Type(SidType::Fn {
    args: None,
    ret:  Some(vec![SidType::Bool]),
  })]);
}

// ── chaining ──────────────────────────────────────────────────────────────────

#[test]
fn typed_args_and_rets_chained_on_substack() {
  let stack = run_snippet("(42) [$int] typed_args ! [$bool] typed_rets !");
  assert_eq!(stack, vec![DataValue::Substack {
    body: vec![ProgramValue::Data(DataValue::Int(42))],
    args: Some(vec![SidType::Int]),
    ret:  Some(vec![SidType::Bool]),
  }]);
}

#[test]
fn typed_args_and_rets_chained_on_fn_type() {
  let stack = run_snippet("fn ! [$int] typed_args ! [$bool] typed_rets !");
  assert_eq!(stack, vec![DataValue::Type(SidType::Fn {
    args: Some(vec![SidType::Int]),
    ret:  Some(vec![SidType::Bool]),
  })]);
}

// ── untyped_args / untyped_rets ───────────────────────────────────────────────

#[test]
fn untyped_args_clears_args_on_substack() {
  let stack = run_snippet("(42) [$int] typed_args ! untyped_args !");
  assert_eq!(stack, vec![DataValue::Substack {
    body: vec![ProgramValue::Data(DataValue::Int(42))],
    args: None,
    ret:  None,
  }]);
}

#[test]
fn untyped_rets_clears_ret_on_substack() {
  let stack = run_snippet("(42) [$int] typed_rets ! untyped_rets !");
  assert_eq!(stack, vec![DataValue::Substack {
    body: vec![ProgramValue::Data(DataValue::Int(42))],
    args: None,
    ret:  None,
  }]);
}

#[test]
fn untyped_args_clears_args_on_fn_type() {
  let stack = run_snippet("fn ! [$int] typed_args ! untyped_args !");
  assert_eq!(stack, vec![DataValue::Type(SidType::Fn { args: None, ret: None })]);
}

#[test]
fn untyped_rets_clears_ret_on_fn_type() {
  let stack = run_snippet("fn ! [$bool] typed_rets ! untyped_rets !");
  assert_eq!(stack, vec![DataValue::Type(SidType::Fn { args: None, ret: None })]);
}
