
use crate::types::*;
use crate::SidType;

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
      DataValue::Bool(v) => v.to_string(),
      DataValue::Str(v) => format!("\"{}\"", v.to_string_lossy()),
      DataValue::Char(v) => format!("\'{}\'", v),
      DataValue::Int(v) => v.to_string(),
      DataValue::Float(v) => v.to_string(),
      DataValue::Substack { body: v, .. } => list_to_syntax(v, "(", ")"),
      DataValue::Script { body: v, .. } => list_to_syntax(v, "<", ">"),
      DataValue::List(v) => list_to_syntax(v, "[", "]"),
      DataValue::Set(v) => list_to_syntax(v, "{", "}"),
      DataValue::Struct(fields) => {
        let inner = fields.iter()
          .map(|(k, v)| format!("{}: {}", k, v.to_syntax()))
          .collect::<Vec<_>>().join(", ");
        format!("{{{}}}", inner)
      },
      DataValue::Map(entries) => {
        let inner = entries.iter()
          .map(|(k, v)| format!("{}: {}", k.to_syntax(), v.to_syntax()))
          .collect::<Vec<_>>().join(", ");
        format!("{{{}}}", inner)
      },
      DataValue::BuiltIn(v) => v.clone(),
      DataValue::Type(v) => v.to_syntax(),
      DataValue::Label(v) => v.clone(),
      DataValue::CFunction(f) => format!("<CFunction {}>", f.name),
      DataValue::Pointer { addr, pointee_ty } =>
        format!("<Pointer 0x{:x} : {}>", addr, pointee_ty.to_syntax()),
      DataValue::CFuncSig(sig) => format!("<CFuncSig {}>", sig.name),
    }
  }
}

impl ToSyntax for ProgramValue {
  fn to_syntax(&self) -> String {
    match self {
      ProgramValue::Data(v) => v.to_syntax(),
      ProgramValue::Invoke => "!".to_owned(),
      ProgramValue::ComptimeInvoke => "@!".to_owned(),
      ProgramValue::Template(v) => v.data.to_syntax(),
      ProgramValue::StackSizeAssert { expected_len, message } => {
        format!("# stack size assert: {} == {} items\n", message, expected_len)
      },
      ProgramValue::CondLoop { cond, body, .. } => {
        let cond_syntax: String = cond.iter().map(|pv| pv.to_syntax()).collect::<Vec<_>>().join(" ");
        let body_syntax: String = body.iter().map(|pv| pv.to_syntax()).collect::<Vec<_>>().join(" ");
        format!("({}) ({}) while_do !", cond_syntax, body_syntax)
      },
    }
  }
}

impl ToSyntax for TemplateData {
  fn to_syntax(&self) -> String {
    match self {
      TemplateData::Substack(v) => list_to_syntax(v, "(#Template", ")"),
      TemplateData::List(v) => list_to_syntax(v, "[#Template", "]"),
      TemplateData::Script(v) => list_to_syntax(v, "<#Template", ">"),
      TemplateData::Set(v) => list_to_syntax(v, "{#Template", "}"),
      TemplateData::Map(pairs) => {
        let mut s = "{#Template".to_owned();
        for (k, v) in pairs {
          s = format!("{}\n {}: {} ", s, k.to_syntax(), v.to_syntax());
        }
        format!("{}\n}}", s)
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

impl ToSyntax for SidType {
  fn to_syntax(&self) -> String {
    match self {
      SidType::Bool  => "bool".to_owned(),
      SidType::Int   => "int".to_owned(),
      SidType::Float => "float".to_owned(),
      SidType::Char  => "char".to_owned(),
      SidType::Str   => "str".to_owned(),
      SidType::Label => "label".to_owned(),
      SidType::Any   => "Any".to_owned(),
      SidType::List(elem)             => format!("{} list @!", elem.to_syntax()),
      SidType::Map { key, value }     => format!("{} {} map @!", key.to_syntax(), value.to_syntax()),
      SidType::Fn { args, ret } => {
        let mut s = "fn".to_owned();
        if let Some(ts) = args {
          let inner = ts.iter().map(|t| t.to_syntax()).collect::<Vec<_>>().join(" ");
          s = format!("{} [{}] typed_args @!", s, inner);
        }
        if let Some(ts) = ret {
          let inner = ts.iter().map(|t| t.to_syntax()).collect::<Vec<_>>().join(" ");
          s = format!("{} [{}] typed_rets @!", s, inner);
        }
        s
      },
      SidType::Pointer(pointee)       => format!("{} ptr @!", pointee.to_syntax()),
      SidType::Literal(v)             => v.to_syntax(),
      SidType::Union(types) => {
        let inner = types.iter().map(|t| t.to_syntax()).collect::<Vec<_>>().join(", ");
        format!("{{{}}}", inner)
      },
      SidType::Struct(fields) => {
        let inner = fields.iter()
          .map(|(name, t)| format!("{}: {}", name, t.to_syntax()))
          .collect::<Vec<_>>().join(", ");
        format!("{{{}}}", inner)
      },
    }
  }
}


