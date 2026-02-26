use super::*;

#[test]
fn parse_with_stack_template() {
    ParseTestFixture {
        input: "$3",
        expected_output: vec![TemplateValue::ParentStackMove(3)],
        expected_consumed: 3,
    }.test();
}

#[test]
fn parse_with_parent_label() {
    ParseTestFixture {
        input: "$label_name",
        expected_output: vec![TemplateValue::ParentLabel("label_name".to_owned())],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_substack() {
    ParseTestFixture {
        input: "(\"data\" 5 $1)",
        expected_output: vec![
            Template::substack((
                vec![
                    DataValue::Str("data".to_owned()).into(),
                    DataValue::Int(5).into(),
                    TemplateValue::ParentStackMove(1),
                ],
                1
            )).into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_script() {
    ParseTestFixture {
        input: "<\"hi\" 5>",
        expected_output: vec![
            Template::script((
                vec![
                    DataValue::Str("hi".to_owned()).into(),
                    DataValue::Int(5).into(),
                ],
                0
            )).into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_list() {
    ParseTestFixture {
        input: "[\"data\" 5 $1]",
        expected_output: vec![
            Template::list((
                vec![
                    DataValue::Str("data".to_owned()).into(),
                    DataValue::Int(5).into(),
                    TemplateValue::ParentStackMove(1),
                ],
                1
            )).into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_set() {
    ParseTestFixture {
        input: "{1, 2, 3}",
        expected_output: vec![
            Template::set((
                vec![
                    DataValue::Int(1).into(),
                    DataValue::Int(2).into(),
                    DataValue::Int(3).into(),
                ],
                0
            )).into(),
        ],
        expected_consumed: 0,
    }.test();
}

#[test]
fn parse_map() {
    ParseTestFixture {
        input: "{x: 1, y: 2}",
        expected_output: vec![
            Template::map(
                vec![
                    (DataValue::Label("x".to_owned()).into(), DataValue::Int(1).into()),
                    (DataValue::Label("y".to_owned()).into(), DataValue::Int(2).into()),
                ],
                0
            ).into(),
        ],
        expected_consumed: 0,
    }.test();
}
