use super::*;

#[test]
fn parse_string() {
    ParseTestFixture {
        input: "\"hi\" \"there\"",
        expected_output: vec![
            DataValue::Str("hi".to_owned()).into(),
            DataValue::Str("there".to_owned()).into(),
        ],
        expected_consumed: 0,
    }.test();
}
