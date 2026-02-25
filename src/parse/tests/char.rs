use super::*;

#[test]
fn parse_char() {
    ParseTestFixture {
        input: "'H' '👮‍♀️'",
        expected_output: vec![
            RealValue::Char("H".to_owned()).into(),
            RealValue::Char("👮‍♀️".to_owned()).into(),
        ],
        expected_consumed: 0,
    }.test();
}
