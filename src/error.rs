#[derive(Debug,PartialEq)]
pub enum AsaErrorKind {
  UndefinedFunction,
  VariableNotDefined(String),
  DivisionByZero,
  NumberOverflow,
  NumberUnderflow,
  Generic(String),  
}