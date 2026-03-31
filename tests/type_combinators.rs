/// Integration tests for the `require @!` and `exclude @!` type combinator built-ins.
///
/// `require @!` — value must match BOTH the base type and the constraint.
/// `exclude @!` — value must match the base type but must NOT match the forbidden value/type.
///
/// Both are comptime constructors; they accept `DataValue::Type` arguments (or plain
/// values wrapped as `SidType::Literal`) and return a `DataValue::Type` usable as a
/// match pattern.
///
/// Argument convention:
/// - Type arguments to `require @!` / `exclude @!` use `$`-prefixed labels
///   inside `@{...}` comptime maps (`$types.str` etc.) so they are resolved
///   eagerly at comptime render time and arrive as `DataValue::Type` values.
///   A bare label (no `$`) is NOT resolved and becomes a literal label constraint.
/// - The whole match-case map is `@{...}` so type expressions are fully
///   computed at comptime and the result is a concrete map at runtime.
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
    interpret_one(
      &mut exe_state.data_stack,
      &mut exe_state.program_stack,
      &mut exe_state.local_scope,
      &mut exe_state.scope_stack,
      &mut exe_state.global_state,
      &builtins,
    );
  }
  exe_state.data_stack.into_iter().filter_map(|tv| {
    if let TemplateValue::Literal(ProgramValue::Data(v)) = tv { Some(v) } else { None }
  }).collect()
}

// ── require @! ────────────────────────────────────────────────────────────────

/// A value matching both the base type and the exact-value constraint hits the require arm.
/// `$types.int 42 require @!` matches only the int value 42.
#[test]
fn require_matches_both() {
  let stack = run_snippet(
    "42 @{$types.int 42 require @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(true)]);
}

/// A value matching the base type but not the exact-value constraint misses the require arm.
#[test]
fn require_misses_when_constraint_fails() {
  let stack = run_snippet(
    "99 @{$types.int 42 require @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// A value not matching the base type misses the require arm even if the literal would.
#[test]
fn require_misses_when_base_fails() {
  let stack = run_snippet(
    r#""hello" @{$types.int 42 require @!: (true), $types.any: (false)} match !"#
  );
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// `$types.any  $types.int  require @!` is equivalent to `$types.int` (any AND int = int).
#[test]
fn require_any_and_int_matches_int() {
  let stack = run_snippet(
    "7 @{$types.any $types.int require @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(true)]);
}

/// `$types.any  $types.int  require @!` does not match a non-int.
#[test]
fn require_any_and_int_rejects_str() {
  let stack = run_snippet(
    r#""hi" @{$types.any $types.int require @!: (true), $types.any: (false)} match !"#
  );
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// Disjoint types — `$types.int  $types.str  require @!` matches nothing.
#[test]
fn require_disjoint_types_matches_nothing() {
  let stack = run_snippet(
    r#"42 @{$types.int $types.str require @!: (true), $types.any: (false)} match !"#
  );
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

// ── exclude @! ────────────────────────────────────────────────────────────────

/// A value matching the base type but not the forbidden literal hits the exclude arm.
/// `$types.int  0  exclude @!` matches any int except 0.
#[test]
fn exclude_matches_base_not_forbidden() {
  let stack = run_snippet(
    "5 @{$types.int 0 exclude @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(true)]);
}

/// The forbidden value itself is rejected.
#[test]
fn exclude_rejects_forbidden_value() {
  let stack = run_snippet(
    "0 @{$types.int 0 exclude @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// `$types.any  0  exclude @!` acts as "any value except 0".
#[test]
fn exclude_any_except_literal() {
  let stack = run_snippet(
    r#""hello" @{$types.any 0 exclude @!: (true), $types.any: (false)} match !"#
  );
  assert_eq!(stack, vec![DataValue::Bool(true)]);
}

/// The forbidden literal is rejected from `$types.any  forbidden  exclude @!`.
#[test]
fn exclude_any_rejects_forbidden_literal() {
  let stack = run_snippet(
    "0 @{$types.any 0 exclude @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// `$types.str  $types.int  exclude @!` — int is the forbidden type.
/// A str matches because it IS $types.str and is NOT $types.int.
#[test]
fn exclude_type_as_forbidden_allows_base() {
  let stack = run_snippet(
    r#""hi" @{$types.str $types.int exclude @!: (true), $types.any: (false)} match !"#
  );
  assert_eq!(stack, vec![DataValue::Bool(true)]);
}

/// A value not matching the base type misses the exclude arm.
#[test]
fn exclude_base_mismatch_falls_through() {
  let stack = run_snippet(
    r#"42 @{$types.str $types.int exclude @!: (true), $types.any: (false)} match !"#
  );
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

// ── null exclusion (canonical use case) ──────────────────────────────────────

/// `$types.any  $types.null  exclude @!` matches any non-null value.
/// `$types.null` is `$`-prefixed so it renders as the actual null pointer value,
/// which is non-Type and therefore wrapped as `SidType::Literal`.
#[test]
fn exclude_any_except_null_matches_int() {
  let stack = run_snippet(
    "42 @{$types.any $types.null exclude @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(true)]);
}

/// The null pointer itself is rejected by the null-exclusion type.
/// `$types.null` is used for the value-to-match so it renders as the actual null pointer.
#[test]
fn exclude_any_except_null_rejects_null() {
  let stack = run_snippet(
    "$types.null @{$types.any $types.null exclude @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

// ── label literal constraints (the bug fix) ───────────────────────────────────

/// A bare label argument is NOT resolved; it becomes a `SidType::Literal(Label(…))`
/// constraint.  This makes it possible to express "value must be the label `foo`".
#[test]
fn require_bare_label_matches_exact_label() {
  let stack = run_snippet(
    "foo @{$types.any foo require @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(true)]);
}

/// A different label does not satisfy a literal-label constraint.
#[test]
fn require_bare_label_rejects_different_label() {
  let stack = run_snippet(
    "bar @{$types.any foo require @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// `exclude @!` with a bare label forbids exactly that label value.
#[test]
fn exclude_bare_label_forbids_exact_label() {
  let stack = run_snippet(
    "foo @{$types.any foo exclude @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(false)]);
}

/// Other label values pass through the label exclusion.
#[test]
fn exclude_bare_label_allows_other_label() {
  let stack = run_snippet(
    "bar @{$types.any foo exclude @!: (true), $types.any: (false)} match !"
  );
  assert_eq!(stack, vec![DataValue::Bool(true)]);
}
