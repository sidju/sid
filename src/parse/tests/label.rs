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
            RealValue::Bool(true).into(),
            RealValue::Bool(false).into(),
        ],
        expected_consumed: 0,
    }.test();
}
