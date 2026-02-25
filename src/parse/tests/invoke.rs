use super::*;

#[test]
fn parse_invoke() {
    ParseTestFixture {
        input: "(5)!",
        expected_output: vec![
            Template::substack((
                vec![RealValue::Int(5).into()],
                0
            )).into(),
            ProgramValue::Invoke.into(),
        ],
        expected_consumed: 0,
    }.test();
}
