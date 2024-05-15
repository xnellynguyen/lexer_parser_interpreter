extern crate asalang;
extern crate nom;
use std::io::Write;

use asalang::*;
use nom::IResult;

macro_rules! test_fragment {
  ($func:ident, $test:tt, $expected:expr) => (
    #[test]
    fn $func() -> Result<(),AsaErrorKind> {
      let tokens = lex($test);
      match program(tokens) {
        Ok((tokens, tree)) => {
          assert_eq!(tokens.is_done(), true); // Check that input token stream is fully parsed
          let mut interpreter = Interpreter::new();
          let result = interpreter.exec(&tree);
          std::io::stdout().flush();
          assert_eq!(result, $expected);
          Ok(())
        },
        Err(e) => Err(AsaErrorKind::Generic(format!("{:?}",e))),
      }
    }
  )
}

macro_rules! test_program {
  ($func:ident, $test:tt, $expected:expr) => (
    #[test]
    fn $func() -> Result<(),AsaErrorKind> {
      let tokens = lex($test);
      match program(tokens) {
        Ok((tokens, tree)) => {
          assert_eq!(tokens.is_done(), true); // Check that input token stream is fully parsed
          let mut interpreter = Interpreter::new();
          let compile_result = interpreter.exec(&tree)?;
          let main_result = interpreter.start_main(vec![]);
          assert_eq!(main_result, $expected);
          Ok(())
        },
        Err(e) => Err(AsaErrorKind::Generic(format!("{:?}",e))),
      }
    }
  )
}

// Test interpreter fragments (no main function)
test_fragment!(interpreter_numeric, r#"123"#, Ok(Value::Number(123)));
test_fragment!(interpreter_string, r#""helloworld""#, Ok(Value::String("helloworld".to_string())));
test_fragment!(interpreter_bool_true, r#"true"#, Ok(Value::Bool(true)));
test_fragment!(interpreter_bool_false, r#"false"#, Ok(Value::Bool(false)));
test_fragment!(interpreter_identifier, r#"x"#, Err(AsaErrorKind::UndefinedFunction));
test_fragment!(interpreter_function_call, r#"foo()"#, Err(AsaErrorKind::UndefinedFunction));
test_fragment!(interpreter_function_call_one_arg, r#"foo(a)"#, Err(AsaErrorKind::UndefinedFunction));
test_fragment!(interpreter_function_call_more_args, r#"foo(a,b,c)"#, Err(AsaErrorKind::UndefinedFunction));
test_fragment!(interpreter_variable_define, r#"let x = 123;"#, Ok(Value::Number(123)));
test_fragment!(interpreter_variable_init, r#"let x = 1;"#, Ok(Value::Number(1)));
test_fragment!(interpreter_variable_bool, r#"let bool = true;"#, Ok(Value::Bool(true)));
test_fragment!(interpreter_variable_string, r#"let string = "HelloWorld";"#, Ok(Value::String("HelloWorld".to_string())));
test_fragment!(interpreter_variable_init_no_space, r#"let x=1;"#, Ok(Value::Number(1)));
test_fragment!(interpreter_math, r#"1 + 1"#, Ok(Value::Number(2)));
test_fragment!(interpreter_math_no_space, r#"1-1"#, Ok(Value::Number(0)));
test_fragment!(interpreter_math_multiply, r#"2 + 4"#, Ok(Value::Number(6)));
test_fragment!(interpreter_assign_math, r#"let x = 1 + 1;"#, Ok(Value::Number(2)));
test_fragment!(interpreter_assign_function, r#"let x = foo();"#, Err(AsaErrorKind::UndefinedFunction));
test_fragment!(interpreter_assign_function_arguments, r#"let x = foo(a,b,c);"#, Err(AsaErrorKind::UndefinedFunction));

// Test full programs
test_program!(interpreter_define_function, r#"fn main(){return foo();} fn foo(){return 5;}"#, Ok(Value::Number(5)));
test_program!(interpreter_define_function_args, r#"fn main(){return foo(1,2);} fn foo(a,b){return a+b;}"#, Ok(Value::Number(3)));
test_program!(interpreter_define_function_more_statement, r#"fn main() {
  return foo();
}
fn foo(){
  let x = 5;
  return x;
}"#, Ok(Value::Number(5)));
test_program!(interpreter_define_full_program, r#"fn foo(a,b,c) {
  let x = a + 1;     
  let y = bar(c + b); 
  return x + y;
}

fn bar(a) {
  return a + 3;
}

fn main() {
  return foo(1,2,3);  
}"#, Ok(Value::Number(10)));

// Additional tests

// Nested function calls
test_program!(interpreter_nested_function_calls, r#"fn main(){return buzz();} fn buzz(){return foo();} fn foo(){return 40;}"#, Ok(Value::Number(40)));

// Function return boolean
test_program!(interpreter_function_return_bool, r#"fn main(){return true;}"#, Ok(Value::Bool(true)));

// Nested function return math expression result
test_program!(interpreter_function_return_variable, r#"fn main(){return foo();} fn foo(){let x = 4+4; return x;}"#, Ok(Value::Number(8)));

// Function with multiple statements returning math result
test_program!(interpreter_multiple_statements, r#"fn main(){return foo();} fn foo(){let x = 5; let y = 10; return x + y;}"#, Ok(Value::Number(15)));

// Full program with multiple functions and math expressions
test_program!(interpreter_full_program_2, r#"fn manipulation(a,b) {
  let x = a + b;     
  let y = x + a; 
  return y;
}

fn main() {
  return manipulation(5,10);  
}"#, Ok(Value::Number(20)));

// 10 additional tests for final

// 1. Test compare numbers
test_fragment!(interpreter_compare_digits, r#"0 >= 1"#, Ok(Value::Bool(false)));

// 2. Test assigning boolean to a variable and compare boolean
test_fragment!(interpreter_assign_and_compare_boolean, r#"
let x = true;
x == true
"#, Ok(Value::Bool(true)));

// 3. Nested conditional/comparison
test_fragment!(interpreter_nested_comparison, r#"0 >= 1 == false"#, Ok(Value::Bool(true)));

// 4. Test assigning math value to variable and compare
// 10 < 5 ==> false 
// false != false ==> false
test_program!(interpreter_nested_assign_and_compare_boolean, r#"
fn main() {
  let x = 10;
  let y = 5;
  let result = x < y != false; 
}
"#, Ok(Value::Bool(false)));

// 5. Assign boolean and compare with another variable holding comparison
// true != false ==> true
test_program!(interpreter_nested_assign_and_compare_boolean_and_math, r#"
fn main() {
  let x = true;
  let y = 5 > 5;
  let result = x != y;
}
"#, Ok(Value::Bool(true)));

// 6. Test an invalid case
test_fragment!(interpreter_invalid_comparison, r#"1 < false"#, Err(AsaErrorKind::Generic("Mismatched types in conditional expression".to_string())));

// 7. Test another invalid case
test_fragment!(interpreter_invalid_comparison_2, r#"10 - false"#, Err(AsaErrorKind::Generic("MathOperationError".to_string())));

// 8. Test full program with multiple functions and comparison
test_program!(interpreter_nested_function_calculations, r#"
fn calculate(a, b) {
  return a + b;
}

fn main() {
  let x = calculate(15, 16);
  let y = calculate(5, 5);
  let result = x > y == true;
}
"#, Ok(Value::Bool(true)));

// 9. Invalid math operation due to type mismatch test
test_program!(interpreter_mixed_type_error_handling, r#"
fn main() {
  let x = 5 == 5;
  let y = 10;
  let result = x + y;  
}
"#, Err(AsaErrorKind::Generic("MathOperationError".to_string())));

// 10. Long program test
test_program!(interpreter_function_result_comparison, r#"
fn add(a, b) {
  return a + b;
}

fn main() {
  let x = add(20, 10);
  let result = x > 20; 
}
"#, Ok(Value::Bool(true)));