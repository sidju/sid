use super::*;

#[test]
fn parse_integer() {
    ParseTestFixture {
        input: "-10 500000",
        expected_output: vec![
            RealValue::Int(-10).into(),
            RealValue::Int(500000).into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_float() {
    ParseTestFixture {
        input: "-10.5 0.66",
        expected_output: vec![
            RealValue::Float(-10.5).into(),
            RealValue::Float(0.66).into(),
        ],
        expected_consumed: 0,
    }.test();
}
