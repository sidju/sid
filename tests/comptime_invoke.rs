use sid::*;

use std::collections::HashMap;

// ── Mock builtins ────────────────────────────────────────────────────────────

/// arg=1, ret=1: doubles an Int.
#[derive(Debug)]
struct MockDouble;
impl InterpretBuiltIn for MockDouble {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 1 }
  fn execute(&self, arg: Option<DataValue>, _scope: &HashMap<String, DataValue>)
    -> anyhow::Result<Option<DataValue>>
  {
    match arg {
      Some(DataValue::Int(n)) => Ok(Some(DataValue::Int(n * 2))),
      other => anyhow::bail!("MockDouble: expected Int, got {:?}", other),
    }
  }
}

/// arg=1, ret=0: drops its argument.
#[derive(Debug)]
struct MockDrop;
impl InterpretBuiltIn for MockDrop {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 0 }
  fn execute(&self, _arg: Option<DataValue>, _scope: &HashMap<String, DataValue>)
    -> anyhow::Result<Option<DataValue>>
  {
    Ok(None)
  }
}

/// arg=0, ret=1: always pushes Int(42).
#[derive(Debug)]
struct MockConst;
impl InterpretBuiltIn for MockConst {
  fn arg_count(&self) -> u8 { 0 }
  fn return_count(&self) -> u8 { 1 }
  fn execute(&self, _arg: Option<DataValue>, _scope: &HashMap<String, DataValue>)
    -> anyhow::Result<Option<DataValue>>
  {
    Ok(Some(DataValue::Int(42)))
  }
}

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

fn comptime_invoke() -> TemplateValue { ProgramValue::ComptimeInvoke.into() }

fn label(s: &str) -> TemplateValue { DataValue::Label(s.to_owned()).into() }

// ── Pass-through tests ────────────────────────────────────────────────────────

#[test]
fn passthrough_plain_values() {
  ComptimePassFixture {
    input: vec![
      DataValue::Int(1).into(),
      DataValue::Str("hi".to_owned()).into(),
    ],
    expected_output: vec![
      DataValue::Int(1).into(),
      DataValue::Str("hi".to_owned()).into(),
    ],
  }.test(&no_builtins());
}

#[test]
fn passthrough_runtime_invoke() {
  // A plain ! should survive the comptime pass untouched.
  ComptimePassFixture {
    input: vec![
      DataValue::Int(5).into(),
      label("something"),
      ProgramValue::Invoke.into(),
    ],
    expected_output: vec![
      DataValue::Int(5).into(),
      label("something"),
      ProgramValue::Invoke.into(),
    ],
  }.test(&no_builtins());
}

#[test]
fn passthrough_parent_refs() {
  // ParentStackMove and ParentLabel entries pass through untouched.
  ComptimePassFixture {
    input: vec![
      TemplateValue::ParentStackMove(1),
      TemplateValue::ParentLabel("foo".to_owned()),
    ],
    expected_output: vec![
      TemplateValue::ParentStackMove(1),
      TemplateValue::ParentLabel("foo".to_owned()),
    ],
  }.test(&no_builtins());
}

// ── @! invocation tests ───────────────────────────────────────────────────────

#[test]
fn comptime_invoke_one_arg_one_return() {
  let double = MockDouble;
  let mut builtins: HashMap<&str, &dyn InterpretBuiltIn> = HashMap::new();
  builtins.insert("double", &double);

  ComptimePassFixture {
    input: vec![DataValue::Int(5).into(), label("double"), comptime_invoke()],
    expected_output: vec![DataValue::Int(10).into()],
  }.test(&builtins);
}

#[test]
fn comptime_invoke_one_arg_zero_return() {
  let drop = MockDrop;
  let mut builtins: HashMap<&str, &dyn InterpretBuiltIn> = HashMap::new();
  builtins.insert("drop", &drop);

  ComptimePassFixture {
    input: vec![DataValue::Int(5).into(), label("drop"), comptime_invoke()],
    expected_output: vec![],
  }.test(&builtins);
}

#[test]
fn comptime_invoke_zero_arg_one_return() {
  let c = MockConst;
  let mut builtins: HashMap<&str, &dyn InterpretBuiltIn> = HashMap::new();
  builtins.insert("const", &c);

  ComptimePassFixture {
    input: vec![label("const"), comptime_invoke()],
    expected_output: vec![DataValue::Int(42).into()],
  }.test(&builtins);
}

#[test]
fn comptime_invoke_leaves_surrounding_stack_intact() {
  // Values before and after the @! site should be untouched.
  let double = MockDouble;
  let mut builtins: HashMap<&str, &dyn InterpretBuiltIn> = HashMap::new();
  builtins.insert("double", &double);

  ComptimePassFixture {
    input: vec![
      DataValue::Str("before".to_owned()).into(),
      DataValue::Int(3).into(),
      label("double"),
      comptime_invoke(),
      DataValue::Str("after".to_owned()).into(),
    ],
    expected_output: vec![
      DataValue::Str("before".to_owned()).into(),
      DataValue::Int(6).into(),
      DataValue::Str("after".to_owned()).into(),
    ],
  }.test(&builtins);
}

// ── Recursion into template bodies ───────────────────────────────────────────

#[test]
fn recurses_into_substack_body() {
  let double = MockDouble;
  let mut builtins: HashMap<&str, &dyn InterpretBuiltIn> = HashMap::new();
  builtins.insert("double", &double);

  let input_body = Template::substack((
    vec![DataValue::Int(5).into(), label("double"), comptime_invoke()],
    0,
  ));
  let expected_body = Template::substack((
    vec![DataValue::Int(10).into()],
    0,
  ));
  ComptimePassFixture {
    input: vec![TemplateValue::Literal(ProgramValue::Template(input_body))],
    expected_output: vec![TemplateValue::Literal(ProgramValue::Template(expected_body))],
  }.test(&builtins);
}

#[test]
fn recurses_into_list_body() {
  let double = MockDouble;
  let mut builtins: HashMap<&str, &dyn InterpretBuiltIn> = HashMap::new();
  builtins.insert("double", &double);

  let input_body = Template::list((
    vec![DataValue::Int(3).into(), label("double"), comptime_invoke()],
    0,
  ));
  let expected_body = Template::list((
    vec![DataValue::Int(6).into()],
    0,
  ));
  ComptimePassFixture {
    input: vec![TemplateValue::Literal(ProgramValue::Template(input_body))],
    expected_output: vec![TemplateValue::Literal(ProgramValue::Template(expected_body))],
  }.test(&builtins);
}

#[test]
fn recurses_into_script_body() {
  let double = MockDouble;
  let mut builtins: HashMap<&str, &dyn InterpretBuiltIn> = HashMap::new();
  builtins.insert("double", &double);

  let input_body = Template::script((
    vec![DataValue::Int(7).into(), label("double"), comptime_invoke()],
    0,
  ));
  let expected_body = Template::script((
    vec![DataValue::Int(14).into()],
    0,
  ));
  ComptimePassFixture {
    input: vec![TemplateValue::Literal(ProgramValue::Template(input_body))],
    expected_output: vec![TemplateValue::Literal(ProgramValue::Template(expected_body))],
  }.test(&builtins);
}

// ── Error cases ───────────────────────────────────────────────────────────────

#[test]
fn error_comptime_invoke_unknown_function() {
  ComptimeErrorFixture {
    input: vec![DataValue::Int(5).into(), label("nonexistent"), comptime_invoke()],
  }.test(&no_builtins());
}

#[test]
fn error_comptime_invoke_unrendered_template_as_arg() {
  // Argument to @! is an unrendered runtime template — must be a hard error.
  let double = MockDouble;
  let mut builtins: HashMap<&str, &dyn InterpretBuiltIn> = HashMap::new();
  builtins.insert("double", &double);

  let unrendered = Template::substack((vec![], 0)); // comptime: false
  ComptimeErrorFixture {
    input: vec![
      TemplateValue::Literal(ProgramValue::Template(unrendered)),
      label("double"),
      comptime_invoke(),
    ],
  }.test(&builtins);
}

#[test]
fn error_comptime_invoke_parent_ref_as_arg() {
  // A ParentStackMove on the stack cannot be used as a @! argument.
  let double = MockDouble;
  let mut builtins: HashMap<&str, &dyn InterpretBuiltIn> = HashMap::new();
  builtins.insert("double", &double);

  ComptimeErrorFixture {
    input: vec![
      TemplateValue::ParentStackMove(1),
      label("double"),
      comptime_invoke(),
    ],
  }.test(&builtins);
}
