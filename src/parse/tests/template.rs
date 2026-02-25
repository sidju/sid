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
                    RealValue::Str("data".to_owned()).into(),
                    RealValue::Int(5).into(),
                    TemplateValue::ParentStackMove(1),
                ],
                1
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
                    RealValue::Str("data".to_owned()).into(),
                    RealValue::Int(5).into(),
                    TemplateValue::ParentStackMove(1),
                ],
                1
            )).into(),
        ],
        expected_consumed: 0,
    }.test();
}
