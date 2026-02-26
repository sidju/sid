use super::*;

#[test]
fn parse_invoke() {
    ParseTestFixture {
        input: "(5)!",
        expected_output: vec![
            Template::substack((
                vec![DataValue::Int(5).into()],
                0
            )).into(),
            ProgramValue::Invoke.into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_comptime_invoke() {
    ParseTestFixture {
        input: "(5)@!",
        expected_output: vec![
            Template::substack((
                vec![DataValue::Int(5).into()],
                0
            )).into(),
            ProgramValue::ComptimeInvoke.into(),
        ],
        expected_consumed: 0,
    }.test();
}
