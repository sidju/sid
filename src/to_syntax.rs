
use crate::types::*;

pub trait ToSyntax
where 
  Self: Sized {
    fn to_syntax(&self) -> String;
}

fn list_to_syntax<T: ToSyntax>(list: &[T], left_bracket: &str, right_bracket: &str) -> String {
  let mut s = left_bracket.to_owned();
  for item in list.iter() {
    s = format!("{}\n {} ", s, &item.to_syntax());
  }
  format!("{}\n{} ", s, right_bracket)
}

impl ToSyntax for DataValue {
  fn to_syntax(&self) -> String {
    match self {
      DataValue::Real(v) => v.to_syntax(),
      DataValue::Label(v) => v.clone(),
    }
  }
}
impl ToSyntax for RealValue {
  fn to_syntax(&self) -> String {
    match self {
      RealValue::Bool(v) => v.to_string(),
      RealValue::Str(v) => format!("\"{}\"", v.clone()),
      RealValue::Char(v) => format!("\'{}\'", v.clone()),
      RealValue::Int(v) => v.to_string(),
      RealValue::Float(v) => v.to_string(),
      RealValue::Substack(v) => {
        list_to_syntax(v, "{", "}")
      },
      RealValue::List(v) => {
        list_to_syntax(v, "[", "]")
      },
      RealValue::BuiltInFunction(v) => v.clone(),
    }
  }
}

impl ToSyntax for ProgramValue {
  fn to_syntax(&self) -> String {
    match self {
      ProgramValue::Real(v) => v.to_syntax(),
      ProgramValue::Label(v) => v.clone(),
      ProgramValue::Invoke => "!".to_owned(),
      ProgramValue::Template(v) => v.data.to_syntax(),
    }
  }
}

impl ToSyntax for TemplateData {
  fn to_syntax(&self) -> String {
    match self {
      TemplateData::Substack(v) => {
        list_to_syntax(v, "{ #Template", "}")
      },
      TemplateData::List(v) => {
        list_to_syntax(v, "[ #Template", "]")
      },
      TemplateData::Script(_) | TemplateData::Set(_) | TemplateData::Struct(_) => {
        todo!()
      },
    }
  }
}

impl ToSyntax for TemplateValue {
  fn to_syntax(&self) -> String {
      match self {
        TemplateValue::ParentLabel(v) => format!("${}", v),
        TemplateValue::ParentStackMove(v) => format!("${}", v),
        TemplateValue::Literal(v) => v.to_syntax(),
      }
  }
}
