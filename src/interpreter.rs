use crate::parser::Node;
use std::collections::HashMap;
use crate::error::*;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
  String(String),
  Number(i32),
  Bool(bool),
}

type Frame = HashMap<String, Value>;
type Arguments = Node;
type Statements = Node;

#[derive(Debug)]
pub struct Interpreter {
  // Function Table:
  // Key - Function name
  // Value - Vec<Node> arguments, statements
  functions: HashMap<String, (Arguments,Statements)>,
  // Stack:
  // Each element in the stack is a func tion stack frame.
  // Crate a new stack frame on function entry.
  // Pop stack frame on function return.
  // Key - Variable name
  // Value - Variable value
  stack: Vec<Frame>,
}

impl Interpreter {

  pub fn new() -> Interpreter {
    Interpreter {
      functions: HashMap::new(),
      stack: Vec::new(),
    }
  }

  pub fn exec(&mut self, node: &Node) -> Result<Value,AsaErrorKind> {
    //println!("Executing node: {:?}\n", node);
    match node {
      Node::Program{children} => {
        for n in children {
          match n {
            Node::FunctionDefine { .. } =>
            {
              self.exec(n)?;
              //println!("Functions registered: {:?}", self.functions);
            }
            |
            Node::Expression{..} |
            Node::VariableDefine{..} |
            Node::String{..} |
            Node::Number{..} |
            Node::Bool{..} => {
              return self.exec(n);
            }
            _ => unreachable!(),
          }
        }
        Ok(Value::Bool(true))
      },

      Node::MathExpression { name, children } => {
        //println!("Current stack before evaluating MathExpression: {:?}", self.stack);
    
        let mut resolve_operand = |child: &Node| -> Result<Value, AsaErrorKind> {
          match child {
            Node::Identifier { value } => {
              // Get the identifier name as a string
              let identifier = std::str::from_utf8(value).unwrap();
              // Retrieve the value from the stack
              for frame in self.stack.iter().rev() {
                if let Some(val) = frame.get(identifier) {
                  return Ok(val.clone());
                }
              }
              Err(AsaErrorKind::UndefinedFunction)
            }
            _ => self.exec(child), // Evaluate other node types normally
          }
        };
    
        let operand1 = resolve_operand(&children[0])?;
        let operand2 = resolve_operand(&children[1])?;
    
        // Convert operands to displayable strings
        let operand1_str = match &operand1 {
          Value::Number(n) => n.to_string(),
          Value::String(s) => s.clone(),
          Value::Bool(b) => b.to_string(),
        };
    
        let operand2_str = match &operand2 {
          Value::Number(n) => n.to_string(),
          Value::String(s) => s.clone(),
          Value::Bool(b) => b.to_string(),
        };
        //println!("here3\n");
    
        // Type of operation
        let operation = match std::str::from_utf8(name) {
          Ok("add") => "+",
          Ok("sub") => "-",
          _ => return Err(AsaErrorKind::Generic("Unknown operation".into())),
        };
    
        //println!("Evaluating expression: {} {} {}", operand1_str, operation, operand2_str);
    
        // Perform the operation
        let result = match (operand1, operand2, name.as_slice()) {
          (Value::Number(op1), Value::Number(op2), b"add") => Ok(Value::Number(op1 + op2)),
          (Value::Number(op1), Value::Number(op2), b"sub") => Ok(Value::Number(op1 - op2)),
          _ => Err(AsaErrorKind::Generic("MathOperationError".into())),
        };

        result
      }

      Node::ConditionalExpression { name, children } => {
        if children.len() != 2 {
          return Err(AsaErrorKind::Generic("Conditional expression must have exactly two operands".to_string()));
        }
    
        // Helper function to unwrap the expression node and resolve values
        let mut resolve_value = |node: &Node| -> Result<Value, AsaErrorKind> {
          match node {
            Node::Expression { children } if !children.is_empty() => {
              // Assume the first child is the actual value or identifier
              let inner_node = &children[0];
              match inner_node {
                Node::ConditionalExpression { .. } => self.exec(inner_node),
                Node::MathExpression { .. } => self.exec(inner_node),
                Node::Identifier { value } => {
                  let identifier = std::str::from_utf8(value).unwrap();
                  // Search the stack for the identifier
                  for frame in self.stack.iter().rev() {
                    if let Some(val) = frame.get(identifier) {
                      return Ok(val.clone());
                    }
                  }
                  Err(AsaErrorKind::VariableNotDefined(identifier.to_string()))
                },
                _ => self.exec(inner_node),
              }
            },
            _ => Err(AsaErrorKind::Generic("Expected an Expression node as child of ConditionalExpression".to_string())),
          }
        };
    
        let left_result = resolve_value(&children[0])?;
        let right_result = resolve_value(&children[1])?;
    
        match (&left_result, &right_result) {
          (Value::Number(left_val), Value::Number(right_val)) => match std::str::from_utf8(name) {
            Ok("gt_") => Ok(Value::Bool(left_val > right_val)),
            Ok("lt_") => Ok(Value::Bool(left_val < right_val)),
            Ok("gte") => Ok(Value::Bool(left_val >= right_val)),
            Ok("lte") => Ok(Value::Bool(left_val <= right_val)),
            Ok("eq_") => Ok(Value::Bool(left_val == right_val)),
            Ok("neq") => Ok(Value::Bool(left_val != right_val)),
            _ => Err(AsaErrorKind::Generic("Unknown conditional operator".to_string())),
          },
          (Value::Bool(left_bool), Value::Bool(right_bool)) => match std::str::from_utf8(name) {
            Ok("eq_") => Ok(Value::Bool(*left_bool == *right_bool)),
            Ok("neq") => Ok(Value::Bool(*left_bool != *right_bool)),
            _ => Err(AsaErrorKind::Generic("Unsupported operator for boolean comparison".to_string())),
          },
          _ => Err(AsaErrorKind::Generic("Mismatched types in conditional expression".to_string())),
        }
      } 

      // Defines a function that takes some arguments and executes a program based on those arguments. 
      // The code first checks if the function exists, and if it does, it creates a new scope in which to execute the function's statements (push a new Frame onto the interpreter stack). 
      // The code then executes each statement in the function's statements list and returns the result of the function's execution. 
      // You will have to correlate each passed value with the apprpriate variable in the called function. If the wrong number or an wrong type of variable is passed, return an error. 
      // On success, insert the return value of the function (if any) into the appropriate entry of the caller's stack.
      Node::FunctionCall { name, children } => {
        let func_name = std::str::from_utf8(name).unwrap();
        //println!("Executing function call: {}\n", func_name);

        // Print the children at the beginning
        //println!("Input children: {:?}\n", children);

        // Fetch function arguments and body
        let (func_args, func_body) = self.functions
            .get(func_name)
            .map(|(args, body)| (args.clone(), body.clone()))
            .ok_or(AsaErrorKind::UndefinedFunction)?;

        // Print func_args and func_body
        //println!("Function arguments (func_args): {:?}", func_args);
        //println!("Function body (func_body): {:?}", func_body);

        let mut stack_frame = Frame::new();

        // Define persistent empty vectors
        let empty_args: Vec<Node> = Vec::new();
        let actual_args = if let Some(Node::FunctionArguments { children }) = children.first() {
          children
        } else {
          &empty_args
        };

        let expected_args = match func_args {
          Node::FunctionArguments { ref children } => children,
          _ => &empty_args,
        };

        // Compare the number of arguments, but allow zero arguments
        if expected_args.len() != actual_args.len() {
          println!(
            "Function '{}' called with an incorrect number of arguments. Expected {}, got {}",
            func_name, expected_args.len(), actual_args.len()
          );
          return Err(AsaErrorKind::Generic(format!(
            "Function '{}' called with an incorrect number of arguments",
            func_name
          )));
        }

        // Map arguments
        for (arg_node, arg_value_node) in expected_args.iter().zip(actual_args.iter()) {

          let arg_value = if let Node::Expression { children } = arg_value_node {
            if let Some(node) = children.first() {
              match node {
                // Check for math expression
                Node::MathExpression { .. } => self.exec(node)?,
                // Check for conditional expression
                Node::ConditionalExpression { .. } => self.exec(node)?,
                // Other nodes
                _ => self.exec(arg_value_node)?
              }
            } else {
              // No children in expression
              return Err(AsaErrorKind::Generic("Expected an expression node with children".into()));
            }
          } else {
            self.exec(arg_value_node)?
          };

          // Extract the identifier 
          let identifier_node = if let Node::Expression { children } = arg_node {
            if let Some(Node::Identifier { value }) = children.first() {
              value.as_slice()
            } else {
              return Err(AsaErrorKind::Generic("Expected identifier inside expression".into()));
            }
          } else {
            return Err(AsaErrorKind::Generic("Expected expression node".into()));
          };

          let arg_name = std::str::from_utf8(identifier_node).unwrap();
          stack_frame.insert(arg_name.to_string(), arg_value);
        }

        // Push new stack frame
        self.stack.push(stack_frame);
        //println!("Stack after pushing new frame: {:?}", self.stack);

        // Execute body
        let mut final_result = Value::Bool(true);
        if let Node::FunctionStatements { children } = func_body {
          for statement in children {
            final_result = self.exec(&statement)?;
          }
        }

        // Pop the stack frame
        self.stack.pop();
        Ok(final_result)
      }
    
      // Defines a new function based on the elements in the children argument. 
      // The name of the function is retrieved from the node struct, the arguments are the first child, and the statements that define the function are the second child. 
      // A new key-value pair is then inserted into the functions table of the interprer. 
      // If the function was successfully defined, the code returns a Value object with a boolean value of true, otherwise an error is returned.
      Node::FunctionDefine {name, children} => {
        let function_name = std::str::from_utf8(name).unwrap();
        let function = (children[0].clone(), children[1].clone());
        if self.functions.contains_key(function_name) {
          return Err(AsaErrorKind::Generic(format!("Function '{}' redefined", function_name)));
        }
        self.functions.insert(function_name.to_string(), function);
        //println!("Current functions: {:?}\n", self.functions);
        Ok(Value::Bool(true))
      },

      // Calls the exec() method on the first element in the children argument, which recursively evaluates the AST of the program being executed and 
      // returns the resulting value or error message.
      Node::FunctionReturn {children} => {
        if let Some(frame) = self.stack.last() {
          let value = self.exec(&children[0])?;
          return Ok(value.clone());
        }
        Err(AsaErrorKind::Generic("Invalid return".to_string()))
      },

      // Retrieves the value of the identifier from the current frame on the stack. If the variable is defined in the current frame, the code returns its value. If the variable is not defined in the current frame, the code returns an error message.
      Node::Identifier { value } => {
        if let Some(frame) = self.stack.last() {
          if let Some(val) = frame.get(std::str::from_utf8(value).unwrap()) {
            return Ok(val.clone());
          }
        }
        Err(AsaErrorKind::UndefinedFunction)
      },

      // Checks the type of the first element in the children argument and deciding what to do based on that type. 
      // If the type is a VariableDefine or FunctionReturn node, the code runs the run method on that node and returns the result.
      Node::FunctionStatements {children} => {
        match &children[0] {
          Node::VariableDefine {children} => {
            let variable_name = match &children[0] {
              Node::Identifier {value} => String::from_utf8_lossy(value).to_string(),
              _ => return Err(AsaErrorKind::Generic("Invalid variable definition".to_string())),
            };

            let value = self.exec(&children[1])?;
            self.stack.push(HashMap::new());
            let frame = self.stack.last_mut().ok_or(AsaErrorKind::Generic("No frame".to_string()))?;
            frame.insert(variable_name, value.clone());
            Ok(value)
          }
          Node::FunctionReturn {children} => self.exec(&Node::FunctionReturn {children: children.to_vec()}),
          _ => Err(AsaErrorKind::Generic("Invalid statement".to_string())),
        }
      },

      // Defines a new variable by assigning a name and a value to it. 
      // The name is retrieved from the first element of the children argument, and the value is retrieved by running the run method on the second element of the children argument. 
      // The key-value pair is then inserted into the last frame on the stack field of the current runtime object.
      Node::VariableDefine {children} => {
        // Variable name
        let name = match &children[0] {
          Node::Identifier {value} => String::from_utf8_lossy(value).to_string(),
          _ => return Err(AsaErrorKind::Generic("Invalid variable definition".to_string())),
        };
    
        // Variable value
        let value = self.exec(&children[1])?;
        // println!("Assigning Variable: {} = {:?}", name, value);  // Print the variable assignment
        self.stack.push(HashMap::new());
        let frame = self.stack.last_mut().ok_or(AsaErrorKind::Generic("No frame".to_string()))?;
        frame.insert(name, value.clone());
        Ok(value)
      }

      // Evaluate the child node using the exec() method.
      Node::Expression{children} => {
        self.exec(&children[0])
      }
      Node::Number{value} => {
        Ok(Value::Number(*value))
      }
      Node::String{value} => {
        Ok(Value::String(value.clone()))
      }
      Node::Bool{value} => {
        Ok(Value::Bool(*value))
      }
      // Return an error message.
      x => {
        Err(AsaErrorKind::Generic("Error".to_string()))
      },
    }
    //println!("Stack after execution: {:?}", self.stack);
  }

  pub fn start_main(&mut self, arguments: Vec<Node>) -> Result<Value,AsaErrorKind> {
    // This node is equivalent to the following Asa program source code:
    // "main()"
    // It calls the main function with a FunctionArguments node as input.
    let start_main = Node::FunctionCall{name: "main".into(), children: arguments};
    // Call the main function by running this code through the interpreter. 
    self.exec(&start_main)
  }
}