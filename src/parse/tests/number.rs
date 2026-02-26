use super::*;

#[test]
fn parse_integer() {
    ParseTestFixture {
        input: "-10 500000",
        expected_output: vec![
            DataValue::Int(-10).into(),
            DataValue::Int(500000).into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_float() {
    ParseTestFixture {
        input: "-10.5 0.66",
        expected_output: vec![
            DataValue::Float(-10.5).into(),
            DataValue::Float(0.66).into(),
        ],
        expected_consumed: 0,
    }.test();
}
