use super::*;

mod string;
mod char;
mod label;
mod number;
mod template;
mod comment;
mod invoke;
mod comptime_template;

pub struct ParseTestFixture {
    pub input: &'static str,
    pub expected_output: Vec<TemplateValue>,
    pub expected_consumed: usize,
}

impl ParseTestFixture {
    pub fn test(&self) {
        let (out, consumed) = parse_str(self.input).expect("parse failed");
        assert_eq!(self.expected_output, out, "Parsed template didn't match expectations");
        assert_eq!(self.expected_consumed, consumed, "Parsed template consumes unexpected amount of stack entries.");
    }
}
