use super::*;

#[test]
fn parse_comments() {
    ParseTestFixture {
        input: "\"hi\" #not\n \"there\"\n#more comments",
        expected_output: vec![
            RealValue::Str("hi".to_owned()).into(),
            RealValue::Str("there".to_owned()).into(),
        ],
        expected_consumed: 0,
    }.test();
}
