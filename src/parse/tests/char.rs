use super::*;

#[test]
fn parse_char() {
    ParseTestFixture {
        input: "'H' '👮‍♀️'",
        expected_output: vec![
            DataValue::Char("H".to_owned()).into(),
            DataValue::Char("👮‍♀️".to_owned()).into(),
        ],
        expected_consumed: 0,
    }
    .test();
}
