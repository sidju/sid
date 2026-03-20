use super::*;

#[test]
fn parse_string() {
    ParseTestFixture {
        input: "\"hi\" \"there\"",
        expected_output: vec![
            DataValue::Str(std::ffi::CString::new("hi").unwrap()).into(),
            DataValue::Str(std::ffi::CString::new("there").unwrap()).into(),
        ],
        expected_consumed: 0,
    }.test();
}
