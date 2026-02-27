use super::*;

#[test]
fn parse_comptime_substack() {
    ParseTestFixture {
        input: "@(5 \"hi\")",
        expected_output: vec![
            Template::substack((
                vec![
                    DataValue::Int(5).into(),
                    DataValue::Str("hi".to_owned()).into(),
                ],
                0
            )).mark_comptime().into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_comptime_list() {
    ParseTestFixture {
        input: "@[1, 2, 3]",
        expected_output: vec![
            Template::list((
                vec![
                    DataValue::Int(1).into(),
                    DataValue::Int(2).into(),
                    DataValue::Int(3).into(),
                ],
                0
            )).mark_comptime().into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_comptime_set() {
    ParseTestFixture {
        input: "@{1, 2}",
        expected_output: vec![
            Template::set((
                vec![
                    DataValue::Int(1).into(),
                    DataValue::Int(2).into(),
                ],
                0
            )).mark_comptime().into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_comptime_map() {
    ParseTestFixture {
        input: "@{x: 1, y: 2}",
        expected_output: vec![
            Template::map(
                vec![
                    (DataValue::Label("x".to_owned()).into(), DataValue::Int(1).into()),
                    (DataValue::Label("y".to_owned()).into(), DataValue::Int(2).into()),
                ],
                0
            ).mark_comptime().into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_comptime_script() {
    ParseTestFixture {
        input: "@<5 \"hi\">",
        expected_output: vec![
            Template::script((
                vec![
                    DataValue::Int(5).into(),
                    DataValue::Str("hi".to_owned()).into(),
                ],
                0
            )).mark_comptime().into(),
        ],
        expected_consumed: 0,
    }.test();
}

/// Regression: @! must remain comptime-invoke, not be confused with comptime template.
#[test]
fn comptime_invoke_not_affected() {
    ParseTestFixture {
        input: "@!",
        expected_output: vec![ProgramValue::ComptimeInvoke.into()],
        expected_consumed: 0,
    }.test();
}

/// Comptime templates may contain $n substitution slots just like runtime templates.
#[test]
fn parse_comptime_substack_with_parent_ref() {
    ParseTestFixture {
        input: "@($1 5)",
        expected_output: vec![
            Template::substack((
                vec![
                    TemplateValue::ParentStackMove(1),
                    DataValue::Int(5).into(),
                ],
                1
            )).mark_comptime().into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_invalid_at_token() {
    assert!(parse_str("@x").is_err(), "@ followed by non-template non-! should be a parse error");
}
