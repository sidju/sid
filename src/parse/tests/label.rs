use super::*;

#[test]
fn parse_label() {
    ParseTestFixture {
        input: "Hello, world",
        expected_output: vec![
            DataValue::Label("Hello".to_owned()).into(),
            DataValue::Label("world".to_owned()).into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_bool() {
    ParseTestFixture {
        input: "true false",
        expected_output: vec![
            DataValue::Bool(true).into(),
            DataValue::Bool(false).into(),
        ],
        expected_consumed: 0,
    }.test();
}

/// `@` must terminate a label so that `label@!` parses as `label` + `@!`
/// and not as the single label `label@`.
#[test]
fn at_sign_terminates_label() {
    ParseTestFixture {
        input: "null@!",
        expected_output: vec![
            DataValue::Label("null".to_owned()).into(),
            ProgramValue::ComptimeInvoke.into(),
        ],
        expected_consumed: 0,
    }.test();
}
