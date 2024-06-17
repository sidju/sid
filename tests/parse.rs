use sid::*;

pub fn parse_test_fixture(
  input: &str,
  expected_output_template: Vec<TemplateValue>,
  expected_output_stack_entries_consumed: usize,
) {
  let output = parse_str(
    input,
  );
  assert_eq!(
    expected_output_template,
    output.0,
    "Parsed template didn't match expectations"
  );
  assert_eq!(
    expected_output_stack_entries_consumed,
    output.1,
    "Parsed template consumes unexpected amount of stack entries."
  );
}

#[test]
fn parse_string() {
  parse_test_fixture(
    "\"hi\" \"there\"",
    vec![
      RealValue::Str("hi".to_owned()).into(),
      RealValue::Str("there".to_owned()).into(),
    ],
    0
  )
}

#[test]
fn parse_char() {
  parse_test_fixture(
    "'H' '👮‍♀️'",
    vec![
      RealValue::Char("H".to_owned()).into(),
      RealValue::Char("👮‍♀️".to_owned()).into(),
    ],
    0
  )
}

#[test]
fn parse_label() {
  parse_test_fixture(
    "Hello, world",
    vec![
      DataValue::Label("Hello,".to_owned()).into(),
      DataValue::Label("world".to_owned()).into(),
    ],
    0
  )
}

#[test]
fn parse_bool() {
  parse_test_fixture(
    "true false",
    vec![
      RealValue::Bool(true).into(),
      RealValue::Bool(false).into(),
    ],
    0
  )
}

#[test]
fn parse_integer() {
  parse_test_fixture(
    "-10 500000",
    vec![
      RealValue::Int(-10).into(),
      RealValue::Int(500000).into(),
    ],
    0
  )
}

#[test]
fn parse_float() {
  parse_test_fixture(
    "-10.5 0.66",
    vec![
      RealValue::Float(-10.5).into(),
      RealValue::Float(0.66).into(),
    ],
    0
  )
}

#[test]
fn parse_with_stack_template() {
  parse_test_fixture(
    "$3",
    vec![
      TemplateValue::ParentStackMove(3),
    ],
    3
  )
}

#[test]
fn parse_with_parent_label() {
  parse_test_fixture(
    "$label_name",
    vec![
      TemplateValue::ParentLabel("label_name".to_owned()),
    ],
    0
  )
}

#[test]
fn parse_substack() {
  parse_test_fixture(
    "(\"data\" 5 $1)",
    vec![
      Template::substack((
        vec![
          RealValue::Str("data".to_owned()).into(),
          RealValue::Int(5).into(),
          TemplateValue::ParentStackMove(1),
        ],
        1
      )).into(),
    ],
    0
  )
}

#[test]
fn parse_invoke() {
  parse_test_fixture(
    "(5)!",
    vec![
      Template::substack((
        vec![
          RealValue::Int(5).into(),
        ],
        0
      )).into(),
      ProgramValue::Invoke.into(),
    ],
    0
  )
}
