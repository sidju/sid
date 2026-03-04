use std::collections::HashMap;
use std::path::PathBuf;

use sid::*;

// ── Helper to locate test fixtures ───────────────────────────────────────────

fn fixture_header() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/test.h");
    p.to_string_lossy().into_owned()
}

// ── C header parsing tests ────────────────────────────────────────────────────

#[test]
fn parses_double_param_double_return() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    let sqrt_sig = sigs.iter().find(|s| s.name == "sqrt").expect("sqrt not found");
    assert_eq!(sqrt_sig.ret, CType::Double);
    assert_eq!(sqrt_sig.params, vec![CType::Double]);
}

#[test]
fn parses_cstring_param_int_return() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    let puts_sig = sigs.iter().find(|s| s.name == "puts").expect("puts not found");
    assert_eq!(puts_sig.ret, CType::Int);
    assert_eq!(puts_sig.params, vec![CType::CString]);
}

#[test]
fn skips_variadic_functions() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    assert!(
        sigs.iter().all(|s| s.name != "printf"),
        "printf (variadic) should have been skipped"
    );
}

#[test]
fn skips_struct_and_typedef_declarations() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    for sig in &sigs {
        assert_ne!(sig.name, "Foo", "struct should be skipped");
        assert_ne!(sig.name, "MyInt", "typedef should be skipped");
    }
}

#[test]
fn error_on_nonexistent_header() {
    let result = parse_c_header("/nonexistent/path/header.h");
    assert!(result.is_err(), "should error on missing file");
}

// ── link_sigs_to_lib tests ────────────────────────────────────────────────────

#[test]
fn links_sqrt_from_libm() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    let linked = link_sigs_to_lib("libm.so.6", &sigs).expect("link_sigs_to_lib failed");
    let sqrt = linked.iter().find(|s| s.name == "sqrt").expect("sqrt not found");
    assert!(sqrt.lib.is_some(), "sqrt should be linked to libm");
}

#[test]
fn unresolved_symbols_left_unlinked() {
    let sigs = vec![CFuncSig {
        name: "this_function_does_not_exist_xyz".to_owned(),
        ret: CType::Int, params: vec![], lib: None,
    }];
    let linked = link_sigs_to_lib("libm.so.6", &sigs).expect("link_sigs_to_lib failed");
    assert!(linked[0].lib.is_none(), "missing symbols should remain unlinked");
}

#[test]
fn error_on_nonexistent_library() {
    let result = link_sigs_to_lib("/nonexistent/libxyz.so.999", &[]);
    assert!(result.is_err(), "should error on missing library");
}

// ── call_cfuncsig tests ───────────────────────────────────────────────────────

fn get_linked_sqrt() -> CFuncSig {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    let linked = link_sigs_to_lib("libm.so.6", &sigs).expect("link_sigs_to_lib failed");
    linked.into_iter().find(|s| s.name == "sqrt").expect("sqrt not found in libm")
}

#[test]
fn call_sqrt_returns_correct_value() {
    let sqrt_sig = get_linked_sqrt();
    let result = call_cfuncsig(&sqrt_sig, Some(DataValue::Float(9.0)))
        .expect("call_cfuncsig failed");
    match result {
        Some(DataValue::Float(v)) => {
            assert!((v - 3.0).abs() < 1e-9, "sqrt(9.0) should be ~3.0, got {}", v);
        }
        other => panic!("expected DataValue::Float, got {:?}", other),
    }
}

#[test]
fn call_with_wrong_arg_count_errors() {
    let sqrt_sig = get_linked_sqrt();
    // sqrt takes 1 param; pass nothing.
    let result = call_cfuncsig(&sqrt_sig, None);
    assert!(result.is_err(), "wrong arg count should return Err");
}

#[test]
fn call_unlinked_sig_errors() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    let unlinked = sigs.into_iter().find(|s| s.name == "sqrt").expect("sqrt not in header");
    assert!(unlinked.lib.is_none());
    let result = call_cfuncsig(&unlinked, Some(DataValue::Float(4.0)));
    assert!(result.is_err(), "calling unlinked CFuncSig should return Err");
}

// ── CFuncSig invoke via interpret ─────────────────────────────────────────────

#[test]
fn interpret_cfuncsig_in_global_scope() {
    let sqrt_sig = get_linked_sqrt();

    // Place the linked CFuncSig in global scope under its name.
    let mut global_scope: HashMap<String, DataValue> = HashMap::new();
    global_scope.insert("sqrt".to_owned(), DataValue::CFuncSig(sqrt_sig));

    let program = vec![ProgramValue::Invoke];
    let data_stack = vec![
        DataValue::Float(16.0).into(),
        DataValue::Label("sqrt".to_owned()).into(),
    ];
    let builtins = get_interpret_builtins();

    // If this panics the test fails; if it returns we know the call succeeded.
    interpret(program, data_stack, global_scope, &builtins);
}

// ── c_load_header builtin tests ───────────────────────────────────────────────

#[test]
fn c_load_header_returns_struct_of_cfuncsigs() {
    let builtins = get_interpret_builtins();
    let builtin = builtins["c_load_header"];

    let result = builtin
        .execute(Some(DataValue::Str(fixture_header())), &HashMap::new())
        .expect("c_load_header failed");

    match result {
        Some(DataValue::Struct(fields)) => {
            let has_sqrt = fields.iter().any(|(name, val)| {
                name == "sqrt" && matches!(val, DataValue::CFuncSig(s) if s.lib.is_none())
            });
            assert!(has_sqrt, "struct should contain an unlinked CFuncSig named 'sqrt'");
        }
        other => panic!("expected Struct, got {:?}", other),
    }
}

#[test]
fn c_load_header_error_on_wrong_arg() {
    let builtins = get_interpret_builtins();
    let result = builtins["c_load_header"]
        .execute(Some(DataValue::Int(42)), &HashMap::new());
    assert!(result.is_err());
}

// ── c_link_lib builtin tests ──────────────────────────────────────────────────

#[test]
fn c_link_lib_links_cfuncsigs_in_struct() {
    let builtins = get_interpret_builtins();

    // First get the unlinked header struct.
    let header = builtins["c_load_header"]
        .execute(Some(DataValue::Str(fixture_header())), &HashMap::new())
        .expect("c_load_header failed")
        .unwrap();

    // Now link against libm.
    let arg = DataValue::Struct(vec![
        ("header".to_owned(), header),
        ("lib".to_owned(), DataValue::Str("libm.so.6".to_owned())),
    ]);
    let result = builtins["c_link_lib"]
        .execute(Some(arg), &HashMap::new())
        .expect("c_link_lib failed");

    match result {
        Some(DataValue::Struct(fields)) => {
            let sqrt = fields.iter()
                .find(|(name, _)| name == "sqrt")
                .expect("sqrt not in linked struct");
            assert!(
                matches!(&sqrt.1, DataValue::CFuncSig(s) if s.lib.is_some()),
                "sqrt should be linked after c_link_lib"
            );
        }
        other => panic!("expected Struct, got {:?}", other),
    }
}

#[test]
fn c_link_lib_error_on_wrong_arg() {
    let builtins = get_interpret_builtins();
    let result = builtins["c_link_lib"]
        .execute(Some(DataValue::Str("not-a-struct".to_owned())), &HashMap::new());
    assert!(result.is_err());
}

// ── c_load_header is comptime-available ───────────────────────────────────────

#[test]
fn c_load_header_available_at_comptime() {
    let builtins = get_comptime_builtins();
    assert!(builtins.contains_key("c_load_header"), "c_load_header must be a comptime builtin");
    assert!(!builtins.contains_key("c_link_lib"), "c_link_lib must NOT be a comptime builtin");
}

