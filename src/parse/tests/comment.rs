use super::*;

#[test]
fn parse_comments() {
    ParseTestFixture {
        input: "\"hi\" #not\n \"there\"\n#more comments",
        expected_output: vec![
            DataValue::Str(std::ffi::CString::new("hi").unwrap()).into(),
            DataValue::Str(std::ffi::CString::new("there").unwrap()).into(),
        ],
        expected_consumed: 0,
    }.test();
}
