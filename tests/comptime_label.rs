use std::collections::HashMap;

use sid::*;

fn comptime_label(name: &str) -> TemplateValue {
    TemplateValue::ComptimeLabel(name.to_owned())
}

#[test]
fn comptime_label_resolves_from_scope() {
    let mut scope = HashMap::new();
    scope.insert("foo".to_owned(), DataValue::Int(99));

    let input = vec![comptime_label("foo")];
    let result = comptime_pass(input, &HashMap::new(), &mut scope).expect("comptime_pass failed");

    assert_eq!(result, vec![DataValue::Int(99).into()]);
}

#[test]
fn comptime_label_errors_when_missing() {
    let mut scope = HashMap::new();
    let input = vec![comptime_label("nonexistent")];
    assert!(
        comptime_pass(input, &HashMap::new(), &mut scope).is_err(),
        "expected comptime_pass to error on missing @label"
    );
}

#[test]
fn comptime_label_inside_nested_substack() {
    let mut scope = HashMap::new();
    scope.insert("x".to_owned(), DataValue::Int(7));

    let input = vec![TemplateValue::Literal(ProgramValue::Template(Template {
        data: TemplateData::Substack(vec![comptime_label("x")]),
        consumes_stack_entries: 0,
    }))];
    let result = comptime_pass(input, &HashMap::new(), &mut scope).expect("comptime_pass failed");

    assert_eq!(
        result,
        vec![TemplateValue::Literal(ProgramValue::Template(Template {
            data: TemplateData::Substack(vec![DataValue::Int(7).into()]),
            consumes_stack_entries: 0,
        }))]
    );
}

#[test]
fn comptime_label_inside_nested_list() {
    let mut scope = HashMap::new();
    scope.insert("v".to_owned(), DataValue::Int(42));

    let input = vec![TemplateValue::Literal(ProgramValue::Template(Template {
        data: TemplateData::List(vec![comptime_label("v")]),
        consumes_stack_entries: 0,
    }))];
    let result = comptime_pass(input, &HashMap::new(), &mut scope).expect("comptime_pass failed");

    assert_eq!(
        result,
        vec![TemplateValue::Literal(ProgramValue::Template(Template {
            data: TemplateData::List(vec![DataValue::Int(42).into()]),
            consumes_stack_entries: 0,
        }))]
    );
}

#[test]
fn comptime_label_inside_nested_map() {
    let mut scope = HashMap::new();
    scope.insert("t".to_owned(), DataValue::Type(SidType::Int));

    let input = vec![TemplateValue::Literal(ProgramValue::Template(Template {
        data: TemplateData::Map(vec![(
            vec![DataValue::Label("key".to_owned()).into()],
            vec![comptime_label("t")],
        )]),
        consumes_stack_entries: 0,
    }))];
    let result = comptime_pass(input, &HashMap::new(), &mut scope).expect("comptime_pass failed");

    assert_eq!(
        result,
        vec![TemplateValue::Literal(ProgramValue::Template(Template {
            data: TemplateData::Map(vec![(
                vec![DataValue::Label("key".to_owned()).into()],
                vec![DataValue::Type(SidType::Int).into()],
            )]),
            consumes_stack_entries: 0,
        }))]
    );
}

#[test]
fn comptime_label_numeric_is_parse_error() {
    let result = sid::parse_str("@1");
    assert!(
        result.is_err(),
        "expected parse error for @<integer>, got {:?}",
        result
    );
}

#[test]
fn comptime_label_parsed_without_at_sign() {
    // Verify that @types.int parses as ComptimeLabel("types.int"), not ComptimeLabel("@types.int")
    let (parsed, _) = sid::parse_str("(@types.int)").expect("parse failed");
    assert_eq!(
        parsed,
        vec![TemplateValue::Literal(ProgramValue::Template(Template {
            data: TemplateData::Substack(vec![TemplateValue::ComptimeLabel(
                "types.int".to_owned()
            )]),
            consumes_stack_entries: 0,
        }))]
    );
}

#[test]
fn comptime_label_resolves_dotted_name() {
    // @types.int should resolve when "types" is a map with "int" as a field
    let mut types_map = HashMap::new();
    types_map.insert("int".to_owned(), DataValue::Type(SidType::Int));

    let mut scope = HashMap::new();
    scope.insert(
        "types".to_owned(),
        DataValue::Map(vec![(
            DataValue::Label("int".to_owned()),
            DataValue::Type(SidType::Int),
        )]),
    );

    let input = vec![comptime_label("types.int")];
    let result = comptime_pass(input, &HashMap::new(), &mut scope).expect("comptime_pass failed");

    assert_eq!(result, vec![DataValue::Type(SidType::Int).into()]);
}
