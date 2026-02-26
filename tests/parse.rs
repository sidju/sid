use sid::*;

pub fn parse_test_fixture(
  input: &str,
  expected_output_template: Vec<TemplateValue>,
  expected_output_stack_entries_consumed: usize,
) {
  let output = parse_str(input).expect("parse failed");
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
      DataValue::Str("hi".to_owned()).into(),
      DataValue::Str("there".to_owned()).into(),
    ],
    0
  )
}

#[test]
fn parse_char() {
  parse_test_fixture(
    "'H' '👮‍♀️'",
    vec![
      DataValue::Char("H".to_owned()).into(),
      DataValue::Char("👮‍♀️".to_owned()).into(),
    ],
    0
  )
}

#[test]
fn parse_label() {
  parse_test_fixture(
    "Hello, world",
    vec![
      DataValue::Label("Hello".to_owned()).into(),
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
      DataValue::Bool(true).into(),
      DataValue::Bool(false).into(),
    ],
    0
  )
}

#[test]
fn parse_integer() {
  parse_test_fixture(
    "-10 500000",
    vec![
      DataValue::Int(-10).into(),
      DataValue::Int(500000).into(),
    ],
    0
  )
}

#[test]
fn parse_float() {
  parse_test_fixture(
    "-10.5 0.66",
    vec![
      DataValue::Float(-10.5).into(),
      DataValue::Float(0.66).into(),
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
          DataValue::Str("data".to_owned()).into(),
          DataValue::Int(5).into(),
          TemplateValue::ParentStackMove(1),
        ],
        1
      )).into(),
    ],
    0
  )
}

#[test]
fn parse_list() {
  parse_test_fixture(
    "[\"data\" 5 $1]",
    vec![
      Template::list((
        vec![
          DataValue::Str("data".to_owned()).into(),
          DataValue::Int(5).into(),
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
          DataValue::Int(5).into(),
        ],
        0
      )).into(),
      ProgramValue::Invoke.into(),
    ],
    0
  )
}


#[test]
fn parse_comments() {
  parse_test_fixture(
    "\"hi\" #not\n \"there\"\n#more comments",
    vec![
      DataValue::Str("hi".to_owned()).into(),
      DataValue::Str("there".to_owned()).into(),
    ],
    0
  )
}

#[test]
fn parse_script() {
  parse_test_fixture(
    "<\"hi\" 5>",
    vec![
      Template::script((
        vec![
          DataValue::Str("hi".to_owned()).into(),
          DataValue::Int(5).into(),
        ],
        0
      )).into(),
    ],
    0
  )
}

#[test]
fn parse_set() {
  parse_test_fixture(
    "{1, 2, 3}",
    vec![
      Template::set((
        vec![
          DataValue::Int(1).into(),
          DataValue::Int(2).into(),
          DataValue::Int(3).into(),
        ],
        0
      )).into(),
    ],
    0
  )
}

#[test]
fn parse_map() {
  parse_test_fixture(
    "{x: 1, y: 2}",
    vec![
      Template::map(
        vec![
          (DataValue::Label("x".to_owned()).into(), DataValue::Int(1).into()),
          (DataValue::Label("y".to_owned()).into(), DataValue::Int(2).into()),
        ],
        0
      ).into(),
    ],
    0
  )
}

#[test]
fn parse_comptime_invoke() {
  parse_test_fixture(
    "(5)@!",
    vec![
      Template::substack((
        vec![DataValue::Int(5).into()],
        0
      )).into(),
      ProgramValue::ComptimeInvoke.into(),
    ],
    0
  )
}
