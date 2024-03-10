use std::collections::{
  HashMap,
  HashSet,
};

// TODO: Add a filters restriction for all of these which takes list of functions returning bool
//       (if the function returns false for a valid it isn't valid for the restriction)
struct BoolRestrictions {
  not_equal: Option<bool>,
}
struct NumberRestrictions<T> {
  not_equal: Option<T>,
  greater_than: Option<T>,
  less_than: Option<T>,
}
struct CharRestrictions {
}
struct StringRestrictions {
}
struct ContainerRestrictions {
  size_greater_than: Option<usize>,
  size_less_than: Option<usize>,
  does_not_contain: Vec<Value>
}
struct FunctionType {
  argument: Type,
  output: Type,
}

enum Type {
  Nil, // We don't allow restrictions on nil, as none make sense
  Literal(Value), // Only this value is allowed
  Bool(BoolRestrictions),
  Byte(NumberRestrictions<u8>),
  Int(NumberRestrictions<i64>),
  Float(NumberRestrictions<f64>),
  Char(CharRestrictions),
  Str(StringRestrictions),
  // Container types specify allowed contained type and their own restrictions
  List(Box<Self>, ContainerRestrictions),
  Set(Box<Self>, ContainerRestrictions),
  Map{key: Box<Self>, val: Box<Self>, restrictions: ContainerRestrictions},
  // Same for meta-types, but they don't have inherent restrictions
  Struct(HashMap<String, Self>),
  Union(HashSet<Self>),
  Fn(FunctionType),
}

// Separated from MetaValue so they can derive Hash
#[derive(PartialEq, Hash)]
enum HashableValue {
  Nil,
  Bool(bool),
  Byte(u8),
  Int(i64),
  Char(String), // Full grapheme cluster
  Str(String),
}
#[derive(PartialEq)]
enum NonHashableValue {
  Float(f64),
}
enum ContainerValue {
  // Meta types need to keep data with its type instance attached
  List(Vec<(Value, Type)>),
  Set(HashSet<(Value, Type)>),
  // We only allow base values as key, since they derive Hash
  Map(HashMap<HashableValue, (Value, Type)>),
  Struct(HashMap<String, (Value, Type)>),
}
struct OrderedFn {
  argument: Type,
  return: Type,
  operations: Vec<Function>,
}
struct UnorderedFn {
  argument: Type,
  return: Type,
  operations: HashSet<Function>,
}
enum FunctionImplementation {
  BuiltIn(String), // Key to function table
  Ordered(OrderedFn),
  Unordered(UnorderedFn),
}
struct Function {
  type: FunctionType,
  implementation: FunctionImplementation,
}
#[derive(PartialEq)]
enum Value {
  HashableValue,
  NonHashableValue,
  ContainerValue,
  Fn(Function),
}
