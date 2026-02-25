use super::*;

#[test]
fn parse_string() {
    ParseTestFixture {
        input: "\"hi\" \"there\"",
        expected_output: vec![
            RealValue::Str("hi".to_owned()).into(),
            RealValue::Str("there".to_owned()).into(),
        ],
        expected_consumed: 0,
    }.test();
}
