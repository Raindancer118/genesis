use anyhow::{Result, anyhow};
use colored::Colorize;
use inquire::Text;

pub fn run(expression: Option<String>) -> Result<()> {
    println!("{}", "🧮 Calculator".bold().cyan());
    
    let expr = match expression {
        Some(e) => e,
        None => {
            // Interactive mode
            println!("Enter an expression (or 'quit' to exit):");
            loop {
                let input = Text::new(">").prompt()?;
                if input.trim().to_lowercase() == "quit" || input.trim().to_lowercase() == "exit" {
                    break;
                }
                match evaluate(&input) {
                    Ok(result) => println!("{} = {}", input.cyan(), result.to_string().green().bold()),
                    Err(e) => println!("{}: {}", "Error".red().bold(), e),
                }
            }
            return Ok(());
        }
    };
    
    // Single expression mode
    match evaluate(&expr) {
        Ok(result) => {
            println!("{} = {}", expr.cyan(), result.to_string().green().bold());
        },
        Err(e) => {
            return Err(anyhow!("Calculation error: {}", e));
        }
    }
    
    Ok(())
}

fn evaluate(expr: &str) -> Result<f64> {
    let expr = expr.trim();
    
    // Simple expression parser using Reverse Polish Notation
    // Supports: +, -, *, /, ^, sqrt, sin, cos, tan, abs
    
    // Convert to RPN and evaluate
    let tokens = tokenize(expr)?;
    let rpn = shunting_yard(tokens)?;
    eval_rpn(rpn)
}

#[derive(Debug, Clone)]
enum Token {
    Number(f64),
    Operator(char),
    Function(String),
    LeftParen,
    RightParen,
}

fn tokenize(expr: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();
    let mut current_number = String::new();
    let mut current_func = String::new();
    
    while let Some(&ch) = chars.peek() {
        match ch {
            '0'..='9' | '.' => {
                current_number.push(ch);
                chars.next();
            },
            '+' | '-' | '*' | '/' | '^' => {
                if !current_number.is_empty() {
                    tokens.push(Token::Number(current_number.parse()?));
                    current_number.clear();
                }
                if !current_func.is_empty() {
                    tokens.push(Token::Function(current_func.clone()));
                    current_func.clear();
                }
                tokens.push(Token::Operator(ch));
                chars.next();
            },
            '(' => {
                if !current_func.is_empty() {
                    tokens.push(Token::Function(current_func.clone()));
                    current_func.clear();
                }
                tokens.push(Token::LeftParen);
                chars.next();
            },
            ')' => {
                if !current_number.is_empty() {
                    tokens.push(Token::Number(current_number.parse()?));
                    current_number.clear();
                }
                tokens.push(Token::RightParen);
                chars.next();
            },
            'a'..='z' | 'A'..='Z' => {
                if !current_number.is_empty() {
                    tokens.push(Token::Number(current_number.parse()?));
                    current_number.clear();
                }
                current_func.push(ch);
                chars.next();
            },
            ' ' | '\t' => {
                if !current_number.is_empty() {
                    tokens.push(Token::Number(current_number.parse()?));
                    current_number.clear();
                }
                if !current_func.is_empty() {
                    tokens.push(Token::Function(current_func.clone()));
                    current_func.clear();
                }
                chars.next();
            },
            _ => {
                chars.next();
            }
        }
    }
    
    if !current_number.is_empty() {
        tokens.push(Token::Number(current_number.parse()?));
    }
    if !current_func.is_empty() {
        tokens.push(Token::Function(current_func));
    }
    
    Ok(tokens)
}

fn shunting_yard(tokens: Vec<Token>) -> Result<Vec<Token>> {
    let mut output = Vec::new();
    let mut operators = Vec::new();
    
    for token in tokens {
        match token {
            Token::Number(_) => output.push(token),
            Token::Function(_) => operators.push(token),
            Token::Operator(op) => {
                while let Some(top) = operators.last() {
                    match top {
                        Token::Operator(top_op) => {
                            if precedence(*top_op) >= precedence(op) {
                                output.push(operators.pop().unwrap());
                            } else {
                                break;
                            }
                        },
                        Token::Function(_) => {
                            output.push(operators.pop().unwrap());
                        },
                        _ => break,
                    }
                }
                operators.push(Token::Operator(op));
            },
            Token::LeftParen => operators.push(token),
            Token::RightParen => {
                while let Some(top) = operators.pop() {
                    match top {
                        Token::LeftParen => break,
                        _ => output.push(top),
                    }
                }
            },
        }
    }
    
    while let Some(op) = operators.pop() {
        output.push(op);
    }
    
    Ok(output)
}

fn eval_rpn(rpn: Vec<Token>) -> Result<f64> {
    let mut stack = Vec::new();
    
    for token in rpn {
        match token {
            Token::Number(n) => stack.push(n),
            Token::Operator(op) => {
                if stack.len() < 2 {
                    return Err(anyhow!("Invalid expression"));
                }
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                let result = match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' => {
                        if b == 0.0 {
                            return Err(anyhow!("Division by zero"));
                        }
                        a / b
                    },
                    '^' => a.powf(b),
                    _ => return Err(anyhow!("Unknown operator: {}", op)),
                };
                stack.push(result);
            },
            Token::Function(func) => {
                if stack.is_empty() {
                    return Err(anyhow!("Invalid expression"));
                }
                let a = stack.pop().unwrap();
                let result = match func.as_str() {
                    "sqrt" => a.sqrt(),
                    "sin" => a.to_radians().sin(),
                    "cos" => a.to_radians().cos(),
                    "tan" => a.to_radians().tan(),
                    "abs" => a.abs(),
                    "ln" => a.ln(),
                    "log" | "log10" => a.log10(),
                    _ => return Err(anyhow!("Unknown function: {}", func)),
                };
                stack.push(result);
            },
            _ => {},
        }
    }
    
    if stack.len() != 1 {
        return Err(anyhow!("Invalid expression"));
    }
    
    Ok(stack[0])
}

fn precedence(op: char) -> i32 {
    match op {
        '+' | '-' => 1,
        '*' | '/' => 2,
        '^' => 3,
        _ => 0,
    }
}
