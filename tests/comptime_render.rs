use sid::*;

use std::collections::HashMap;

// ── Fixtures ─────────────────────────────────────────────────────────────────

pub struct ComptimePassFixture {
  pub input: Vec<TemplateValue>,
  pub expected_output: Vec<TemplateValue>,
}
impl ComptimePassFixture {
  pub fn test(&self, builtins: &HashMap<&str, &dyn InterpretBuiltIn>) {
    let result = comptime_pass(self.input.clone(), builtins, &HashMap::new())
      .expect("comptime_pass failed unexpectedly");
    assert_eq!(result, self.expected_output, "comptime_pass output didn't match expectations");
  }
}

pub struct ComptimeErrorFixture {
  pub input: Vec<TemplateValue>,
}
impl ComptimeErrorFixture {
  pub fn test(&self, builtins: &HashMap<&str, &dyn InterpretBuiltIn>) {
    assert!(
      comptime_pass(self.input.clone(), builtins, &HashMap::new()).is_err(),
      "expected comptime_pass to return Err but it succeeded"
    );
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn no_builtins<'a>() -> HashMap<&'a str, &'a dyn InterpretBuiltIn> { HashMap::new() }

// ── Pass-through tests ────────────────────────────────────────────────────────

#[test]
fn passthrough_runtime_template_unchanged() {
  // A runtime substack with no @! inside should be passed through as-is.
  let substack = Template::substack((
    vec![DataValue::Int(5).into(), DataValue::Int(3).into()],
    0,
  ));
  ComptimePassFixture {
    input: vec![TemplateValue::Literal(ProgramValue::Template(substack.clone()))],
    expected_output: vec![TemplateValue::Literal(ProgramValue::Template(substack))],
  }.test(&no_builtins());
}

// ── Comptime template eager rendering ────────────────────────────────────────

#[test]
fn comptime_substack_rendered_eagerly() {
  // @(7) renders immediately to DataValue::Substack([Data(Int(7))]).
  let ct = Template::substack((vec![DataValue::Int(7).into()], 0)).mark_comptime();
  ComptimePassFixture {
    input: vec![TemplateValue::Literal(ProgramValue::Template(ct))],
    expected_output: vec![TemplateValue::from(DataValue::Substack(vec![
      ProgramValue::Data(DataValue::Int(7)),
    ]))],
  }.test(&no_builtins());
}

#[test]
fn comptime_list_rendered_eagerly() {
  let ct = Template::list((
    vec![DataValue::Int(1).into(), DataValue::Int(2).into()],
    0,
  )).mark_comptime();
  ComptimePassFixture {
    input: vec![TemplateValue::Literal(ProgramValue::Template(ct))],
    expected_output: vec![TemplateValue::from(DataValue::List(vec![
      DataValue::Int(1),
      DataValue::Int(2),
    ]))],
  }.test(&no_builtins());
}

// ── Error cases ───────────────────────────────────────────────────────────────

#[test]
fn error_comptime_template_consumes_parent_stack_move() {
  // A comptime template that needs 1 parent entry, but it is a ParentStackMove
  // (not concrete) — must error.
  let ct = Template::substack((vec![], 1)).mark_comptime();
  ComptimeErrorFixture {
    input: vec![
      TemplateValue::ParentStackMove(1),
      TemplateValue::Literal(ProgramValue::Template(ct)),
    ],
  }.test(&no_builtins());
}

#[test]
fn error_comptime_template_consumes_unrendered_template() {
  // A comptime template consuming an unrendered runtime template must error.
  let inner = Template::substack((vec![], 0)); // comptime: false
  let ct = Template::substack((vec![], 1)).mark_comptime();
  ComptimeErrorFixture {
    input: vec![
      TemplateValue::Literal(ProgramValue::Template(inner)),
      TemplateValue::Literal(ProgramValue::Template(ct)),
    ],
  }.test(&no_builtins());
}
