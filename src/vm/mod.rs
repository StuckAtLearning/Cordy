use std::io::{BufRead, Write};
use crate::{stdlib, trace};
use crate::compiler::parser::ParserResult;
use crate::stdlib::StdBinding;
use crate::vm::value::Value;

use crate::vm::Opcode::{*};
use crate::vm::RuntimeErrorType::{*};

pub(crate) mod value;


#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Opcode {

    Noop,

    // Flow Control
    // These only peek() the stack
    JumpIfFalse(usize),
    JumpIfFalsePop(usize),
    JumpIfTrue(usize),
    Jump(usize),

    // Stack Operations
    Dupe, // ... x, y, z] -> ... x, y, z, z]
    Pop,

    PushGlobal(usize),
    StoreGlobal(usize),

    // Push
    Nil,
    True,
    False,
    Int(i64),
    Str(String),
    Bound(StdBinding),

    // todo: remove / replace with other things
    Identifier(String),

    // Unary Operators
    UnarySub,
    UnaryLogicalNot,
    UnaryBitwiseNot,

    // Binary Operators
    // Ordered by precedence, highest to lowest
    OpFuncEval(u8),
    OpArrayEval(u8),

    OpMul,
    OpDiv,
    OpMod,
    OpPow,
    OpIs,

    OpAdd,
    OpSub,

    OpLeftShift,
    OpRightShift,

    OpLessThan,
    OpGreaterThan,
    OpLessThanEqual,
    OpGreaterThanEqual,
    OpEqual,
    OpNotEqual,

    OpBitwiseAnd,
    OpBitwiseOr,
    OpBitwiseXor,

    OpFuncCompose,
    OpIndex,

    // Special
    LineNumber(usize),
    Exit,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct RuntimeError {
    pub error: RuntimeErrorType,
    pub lineno: usize,
    ip: usize,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum RuntimeErrorType {
    ValueIsNotFunctionEvaluable(Value),
    BindingIsNotFunctionEvaluable(StdBinding),

    IncorrectNumberOfArguments(StdBinding, u8, u8),

    TypeErrorUnaryOp(Opcode, Value),
    TypeErrorBinaryOp(Opcode, Value, Value),
    TypeErrorBinaryIs(Value, Value),
    TypeErrorCannotConvertToInt(Value),
    TypeErrorCannotCompare(Value, Value),
    TypeErrorFunc1(&'static str, Value),
    TypeErrorFunc2(&'static str, Value, Value),
    TypeErrorFunc3(&'static str, Value, Value, Value),
}



pub struct VirtualMachine<R, W> {
    ip: usize,
    code: Vec<Opcode>,
    stack: Vec<Value>,

    globals: Vec<Value>,

    lineno: usize,

    read: R,
    write: W,
}


impl<R, W> VirtualMachine<R, W> where
    R: BufRead,
    W: Write {

    pub fn new(parser_result: ParserResult, read: R, write: W) -> VirtualMachine<R, W> {
        VirtualMachine {
            ip: 0,
            code: parser_result.code,
            stack: Vec::new(),
            globals: vec!(Value::Nil; parser_result.globals.len()),
            lineno: 0,
            read,
            write,
        }
    }

    pub fn run(self: &mut Self) -> Result<(), RuntimeError> {
        loop {
            let op: &Opcode = self.code.get(self.ip).unwrap();
            self.ip += 1;
            match op {
                // Flow Control
                // All jumps are absolute (because we don't have variable length instructions and it's easy to do so)
                JumpIfFalse(ip) => {
                    trace::trace_interpreter!("jump if false {} -> {}", self.stack.last().unwrap().as_debug_str(), ip);
                    let jump: usize = *ip;
                    let a1: &Value = self.peek(0);
                    if !a1.as_bool() {
                        self.ip = jump;
                    }
                },
                JumpIfFalsePop(ip) => {
                    trace::trace_interpreter!("jump if false {} -> {}", self.stack.last().unwrap().as_debug_str(), ip);
                    let jump: usize = *ip;
                    let a1: Value = self.pop();
                    if !a1.as_bool() {
                        self.ip = jump;
                    }
                },
                JumpIfTrue(ip) => {
                    trace::trace_interpreter!("jump if true {} -> {}", self.stack.last().unwrap().as_debug_str(), ip);
                    let jump: usize = *ip;
                    let a1: &Value = self.peek(0);
                    if a1.as_bool() {
                        self.ip = jump;
                    }
                }
                Jump(ip) => {
                    trace::trace_interpreter!("jump -> {}", ip);
                    self.ip = *ip;
                }

                // Stack Manipulations
                Dupe => {
                    trace::trace_interpreter!("stack dupe {}", self.stack.last().unwrap().as_debug_str());
                    self.push(self.peek(0).clone());
                }
                Pop => {
                    trace::trace_interpreter!("stack pop {}", self.stack.last().unwrap().as_debug_str());
                    self.pop();
                },

                PushGlobal(gid) => {
                    trace::trace_interpreter!("push global {:?}", gid);
                    self.push(self.globals[*gid].clone());
                },
                StoreGlobal(gid) => {
                    trace::trace_interpreter!("store global {:?}", gid);
                    let g: usize = *gid;
                    self.globals[g] = self.pop();
                }


                // Push Operations
                Nil => {
                    trace::trace_interpreter!("push nil");
                    self.push(Value::Nil);
                },
                True => {
                    trace::trace_interpreter!("push true");
                    self.push(Value::Bool(true));
                },
                False => {
                    trace::trace_interpreter!("push false");
                    self.push(Value::Bool(false));
                },
                Int(i) => {
                    trace::trace_interpreter!("push {}", i);
                    self.push(Value::Int(*i));
                },
                Str(s) => {
                    trace::trace_interpreter!("push '{}'", s);
                    self.push(Value::Str(s.clone()))
                }
                Bound(b) => {
                    trace::trace_interpreter!("push {}", Value::Binding(*b).as_debug_str());
                    self.push(Value::Binding(*b));
                },

                // Unary Operators
                UnarySub => {
                    trace::trace_interpreter!("op unary -");
                    let a1: Value = self.pop();
                    match a1 {
                        Value::Int(i1) => self.push(Value::Int(-i1)),
                        v => return self.error(TypeErrorUnaryOp(UnarySub, v)),
                    }
                },
                UnaryLogicalNot => {
                    trace::trace_interpreter!("op unary !");
                    let a1: Value = self.pop();
                    match a1 {
                        Value::Bool(b1) => self.push(Value::Bool(!b1)),
                        v => return self.error(TypeErrorUnaryOp(UnaryLogicalNot, v)),
                    }
                },
                UnaryBitwiseNot => {
                    trace::trace_interpreter!("op unary ~");
                    let a1: Value = self.pop();
                    match a1 {
                        Value::Int(i1) => self.push(Value::Int(!i1)),
                        v => return self.error(TypeErrorUnaryOp(UnaryBitwiseNot, v)),
                    }
                },

                // Binary Operators
                OpMul => {
                    trace::trace_interpreter!("op binary *");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) => self.push(Value::Int(i1 * i2)),
                        (Value::Str(s1), Value::Int(i2)) if i2 > 0 => self.push(Value::Str(s1.repeat(i2 as usize))),
                        (Value::Int(i1), Value::Str(s2)) if i1 > 0 => self.push(Value::Str(s2.repeat(i1 as usize))),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpMul, l, r))
                    }
                },
                OpDiv => {
                    trace::trace_interpreter!("op binary /");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) if i2 != 0 => self.push(Value::Int(i1 / i2)),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpDiv, l, r))
                    }
                },
                OpMod => {
                    trace::trace_interpreter!("op binary %");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) if i2 != 0 => self.push(Value::Int(i1 % i2)),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpMod, l, r))
                    }
                },
                OpPow => {
                    trace::trace_interpreter!("op binary **");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) if i2 > 0 => self.push(Value::Int(i1.pow(i2 as u32))),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpMod, l, r))
                    }
                },
                OpIs => {
                    trace::trace_interpreter!("op binary 'is'");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match a2 {
                        Value::Binding(b) => {
                            let ret: Value = match b {
                                StdBinding::Nil => Value::Bool(a1.is_nil()),
                                StdBinding::Bool => Value::Bool(a1.is_bool()),
                                StdBinding::Int => Value::Bool(a1.is_int()),
                                StdBinding::Str => Value::Bool(a1.is_str()),
                                _ => return self.error(TypeErrorBinaryIs(a1, Value::Binding(b)))
                            };
                            self.push(ret);
                        },
                        _ => return self.error(TypeErrorBinaryIs(a1, a2))
                    }
                }

                OpAdd => {
                    trace::trace_interpreter!("op binary +");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) => self.push(Value::Int(i1 + i2)),
                        (Value::Str(s1), r) => self.push(Value::Str(format!("{}{}", s1, r.as_str()))),
                        (l, Value::Str(s2)) => self.push(Value::Str(format!("{}{}", l.as_str(), s2))),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpAdd, l, r)),
                    }
                },
                OpSub => {
                    trace::trace_interpreter!("op binary -");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) if i2 > 0 => self.push(Value::Int(i1 - i2)),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpSub, l, r))
                    }
                },

                OpLeftShift => {
                    trace::trace_interpreter!("op binary <<");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) => self.push(Value::Int(if i2 >= 0 { i1 << i2 } else {i1 >> (-i2)})),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpLeftShift, l, r))
                    }
                },
                OpRightShift => {
                    trace::trace_interpreter!("op binary >>");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) => self.push(Value::Int(if i2 >= 0 { i1 >> i2 } else {i1 << (-i2)})),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpRightShift, l, r))
                    }
                },

                OpLessThan => {
                    trace::trace_interpreter!("op binary <");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match a1.is_less_than(&a2) {
                        Ok(v) => self.push(Value::Bool(v)),
                        Err(e) => return self.error(e)
                    }
                },
                OpGreaterThan => {
                    trace::trace_interpreter!("op binary >");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match a1.is_less_than_or_equal(&a2) {
                        Ok(v) => self.push(Value::Bool(!v)),
                        Err(e) => return self.error(e)
                    }
                },
                OpLessThanEqual => {
                    trace::trace_interpreter!("op binary <=");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match a1.is_less_than_or_equal(&a2) {
                        Ok(v) => self.push(Value::Bool(v)),
                        Err(e) => return self.error(e)
                    }
                },
                OpGreaterThanEqual => {
                    trace::trace_interpreter!("op binary >=");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match a1.is_less_than(&a2) {
                        Ok(v) => self.push(Value::Bool(!v)),
                        Err(e) => return self.error(e)
                    }
                },
                OpEqual => {
                    trace::trace_interpreter!("op binary ==");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    self.push(Value::Bool(a1.is_equal(&a2)));
                },
                OpNotEqual => {
                    trace::trace_interpreter!("op binary !=");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    self.push(Value::Bool(!a1.is_equal(&a2)));
                },

                OpBitwiseAnd => {
                    trace::trace_interpreter!("op binary &");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) => self.push(Value::Int(i1 & i2)),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpBitwiseAnd, l, r))
                    }
                },
                OpBitwiseOr => {
                    trace::trace_interpreter!("op binary |");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) => self.push(Value::Int(i1 | i2)),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpBitwiseAnd, l, r))
                    }
                },
                OpBitwiseXor => {
                    trace::trace_interpreter!("op binary ^");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::Int(i1), Value::Int(i2)) => self.push(Value::Int(i1 ^ i2)),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpBitwiseAnd, l, r))
                    }
                },

                OpFuncCompose => {
                    trace::trace_interpreter!("op binary .");
                    let f: Value = self.pop();
                    match f {
                        Value::Binding(b) => {
                            // invoke_func_binding() will pop `nargs` arguments off the stack and pass them to the provided function
                            // Unlike `OpFuncEval`, we have already popped the binding off the stack initially
                            match stdlib::invoke(b, 1, self) {
                                Ok(ret) => self.push(ret),
                                Err(e) => return self.error(e),
                            }
                        },
                        Value::PartialBinding(b, nargs) => {
                            // Need to consume the arguments and set up the stack for calling as if all partial arguments were just pushed
                            // Top of the stack contains `argN+1`, and `nargs` contains `[argN, argN-1, ... arg1]`
                            // After this, it should contain `[..., arg1, arg2, ... argN, argN+1]
                            let held: Value = self.pop();
                            let partial_args: u8 = nargs.len() as u8;
                            for arg in nargs.into_iter().rev() {
                                self.push(arg);
                            }
                            self.push(held);

                            // invoke_func_binding() will pop `nargs` arguments off the stack and pass them to the provided function
                            // Unlike `OpFuncEval`, we have already popped the binding off the stack initially
                            match stdlib::invoke(b, partial_args + 1, self) {
                                Ok(ret) => self.push(ret),
                                Err(e) => return self.error(e),
                            }

                        }
                        _ => return self.error(ValueIsNotFunctionEvaluable(f.clone())),
                    }
                },


                OpFuncEval(nargs_borrow) => {
                    trace::trace_interpreter!("op function evaluate n = {}", nargs_borrow);
                    let nargs: u8 = *nargs_borrow;
                    let f: &Value = self.peek(nargs as usize);
                    match f {
                        Value::Binding(b) => {
                            match stdlib::invoke(*b, nargs, self) {
                                Ok(v) => {
                                    self.pop();
                                    self.push(v)
                                },
                                Err(e) => return self.error(e),
                            }
                        },
                        Value::PartialBinding(b, _) => {
                            // Need to consume the arguments and set up the stack for calling as if all partial arguments were just pushed
                            // Surgically extract the binding via std::mem::replace
                            let binding: StdBinding = *b;
                            let i: usize = self.stack.len() - 1 - nargs as usize;
                            let args: Vec<Value> = match std::mem::replace(&mut self.stack[i], Value::Nil) {
                                Value::PartialBinding(_, x) => x,
                                _ => panic!("Stack corruption")
                            };

                            // Splice the args from the binding back into the stack, in the correct order
                            // When evaluating a partial function the vm stack will contain [..., argN+1, ... argM]
                            // The partial args will contain the vector [argN, argN-1, ... arg1]
                            // After this, the vm stack should contain the args [..., arg1, arg2, ... argM]
                            let j: usize = self.stack.len() - nargs as usize;
                            let partial_args: u8 = args.len() as u8;
                            self.stack.splice(j..j, args.into_iter().rev());

                            match stdlib::invoke(binding, nargs + partial_args, self) {
                                Ok(v) => {
                                    self.pop();
                                    self.push(v);
                                },
                                Err(e) => return self.error(e),
                            }
                        }
                        _ => return self.error(ValueIsNotFunctionEvaluable(f.clone())),
                    }
                },

                OpIndex => {
                    trace::trace_interpreter!("op []");
                    let a2: Value = self.pop();
                    let a1: Value = self.pop();
                    match (a1, a2) {
                        (Value::List(l), Value::Int(r)) => self.push((*l).borrow()[r as usize].clone()),
                        (l, r) => return self.error(TypeErrorBinaryOp(OpIndex, l, r))
                    }
                },

                LineNumber(lineno) => self.lineno = *lineno,
                Exit => break,

                _ => panic!("Unimplemented {:?}", op)
            }
        }
        trace::trace_interpreter_stack!(": [{}]", self.stack.iter().rev().map(|t| t.as_debug_str()).collect::<Vec<String>>().join(", "));
        Ok(())
    }

    /// Constructs a `RuntimeError` to be returned from the main VM loop
    fn error<T>(self: &Self, error: RuntimeErrorType) -> Result<T, RuntimeError> {
        Err(RuntimeError {
            error,
            lineno: self.lineno,
            ip: self.ip
        })
    }
}

pub trait IO {
    fn println0(self: &mut Self);
    fn println(self: &mut Self, str: String);
    fn print(self: &mut Self, str: String);
}

impl <R, W> IO for VirtualMachine<R, W> where
    R: BufRead,
    W: Write {
    fn println0(self: &mut Self) { writeln!(&mut self.write, "").unwrap(); }
    fn println(self: &mut Self, str: String) { writeln!(&mut self.write, "{}", str).unwrap(); }
    fn print(self: &mut Self, str: String) { write!(&mut self.write, "{}", str).unwrap(); }
}

pub trait Stack {
    fn peek(self: &Self, offset: usize) -> &Value;
    fn pop(self: &mut Self) -> Value;
    fn push(self: &mut Self, value: Value);
}

impl<R, W> Stack for VirtualMachine<R, W> {
    // ===== Stack Manipulations ===== //

    /// Peeks at the top element of the stack, or an element `offset` down from the top
    fn peek(self: &Self, offset: usize) -> &Value {
        trace::trace_interpreter_stack!(": [{}]", self.stack.iter().rev().map(|t| t.as_debug_str()).collect::<Vec<String>>().join(", "));
        trace::trace_interpreter_stack!("peek({}) -> {}", offset, self.stack[self.stack.len() - 1 - offset].as_debug_str());
        self.stack.get(self.stack.len() - 1 - offset).unwrap()
    }

    /// Pops the top of the stack
    fn pop(self: &mut Self) -> Value {
        trace::trace_interpreter_stack!(": [{}]", self.stack.iter().rev().map(|t| t.as_debug_str()).collect::<Vec<String>>().join(", "));
        trace::trace_interpreter_stack!("pop() -> {}", self.stack.last().unwrap().as_debug_str());
        self.stack.pop().unwrap()
    }

    /// Push a value onto the stack
    fn push(self: &mut Self, value: Value) {
        trace::trace_interpreter_stack!(": [{}]", self.stack.iter().rev().map(|t| t.as_debug_str()).collect::<Vec<String>>().join(", "));
        trace::trace_interpreter_stack!("push({})", value.as_debug_str());
        self.stack.push(value);
    }
}


#[cfg(test)]
mod test {
    use crate::{compiler, ErrorReporter, trace, VirtualMachine};
    use crate::vm::RuntimeError;

    #[test] fn test_str_empty() { run_str("", ""); }
    #[test] fn test_str_hello_world() { run_str("print('hello world!')", "hello world!\n"); }
    #[test] fn test_str_empty_print() { run_str("print()", "\n"); }
    #[test] fn test_str_strings() { run_str("print('first', 'second', 'third')", "first second third\n"); }
    #[test] fn test_str_other_args() { run_str("print(nil, -1, 1, true, false, 'test', print)", "nil -1 1 true false test print\n"); }
    #[test] fn test_str_unary_ops() { run_str("print(-1, --1, ---1, ~3, ~~3, !true, !!true)", "-1 1 -1 -4 3 false true\n"); }
    #[test] fn test_str_add_str() { run_str("print(('a' + 'b') + (3 + 4) + (' hello' + 3) + (' and' + true + nil))", "ab7 hello3 andtruenil\n"); }
    #[test] fn test_str_mul_str() { run_str("print('abc' * 3)", "abcabcabc\n"); }
    #[test] fn test_str_add_sub_mul_div_int() { run_str("print(5 - 3, 12 + 5, 3 * 9, 16 / 3)", "2 17 27 5\n"); }
    #[test] fn test_str_div_mod_int() { run_str("print(3 / 2, 3 / 3, -3 / 2, 10 % 3, 11 % 3, 12 % 3)", "1 1 -1 1 2 0\n"); }
    #[test] fn test_str_div_by_zero() { run_str("print(15 / 0)", "TypeError: Cannot divide '15' of type 'int' and '0' of type 'int'\n  at: line 1 (<test>)\n  at:\n\nprint(15 / 0)\n"); }
    #[test] fn test_str_mod_by_zero() { run_str("print(15 % 0)", "TypeError: Cannot modulo '15' of type 'int' and '0' of type 'int'\n  at: line 1 (<test>)\n  at:\n\nprint(15 % 0)\n"); }
    #[test] fn test_str_left_right_shift() { run_str("print(1 << 10, 16 >> 1, 16 << -1, 1 >> -10)", "1024 8 8 1024\n"); }
    #[test] fn test_str_compare_ints() { run_str("print(1 < 3, -5 < -10, 6 > 7, 6 > 4)", "true false false true\n"); }
    #[test] fn test_str_compare_ints_2() { run_str("print(1 <= 3, -5 < -10, 3 <= 3, 2 >= 2, 6 >= 7, 6 >= 4, 6 <= 6, 8 >= 8)", "true false true true false true true true\n"); }
    #[test] fn test_str_equal_ints() { run_str("print(1 == 3, -5 == -10, 3 != 3, 2 == 2, 6 != 7)", "false false false true true\n"); }
    #[test] fn test_str_compare_bools() { run_str("print(false < false, false < true, true < false, true < true)", "false true false false\n"); }
    #[test] fn test_str_compare_bools_2() { run_str("print(false <= false, false >= true, true >= false, true <= true)", "true false true true\n"); }
    #[test] fn test_str_bitwise_ops() { run_str("print(0b111 & 0b100, 0b1100 | 0b1010, 0b1100 ^ 0b1010)", "4 14 6\n"); }
    #[test] fn test_str_compose() { run_str("print . print", "print\n"); }
    #[test] fn test_str_compose_str() { run_str("'hello world' . print", "hello world\n"); }
    #[test] fn test_str_if_1() { run_str("if 1 < 2 { print('yes') } else { print ('no') }", "yes\n"); }
    #[test] fn test_str_if_2() { run_str("if 1 < -2 { print('yes') } else { print ('no') }", "no\n"); }
    #[test] fn test_str_if_3() { run_str("if true { print('yes') } print('and also')", "yes\nand also\n"); }
    #[test] fn test_str_if_4() { run_str("if 1 < -2 { print('yes') } print('and also')", "and also\n"); }
    #[test] fn test_str_if_5() { run_str("if 0 { print('yes') }", ""); }
    #[test] fn test_str_if_6() { run_str("if 1 { print('yes') }", "yes\n"); }
    #[test] fn test_str_if_7() { run_str("if 'string' { print('yes') }", "yes\n"); }
    #[test] fn test_str_if_8() { run_str("if 1 < 0 { print('yes') } elif 1 { print('hi') } else { print('hehe')", "hi\n"); }
    #[test] fn test_str_if_9() { run_str("if 1 < 0 { print('yes') } elif 2 < 0 { print('hi') } else { print('hehe')", "hehe\n"); }
    #[test] fn test_str_if_10() { run_str("if 1 { print('yes') } elif true { print('hi') } else { print('hehe')", "yes\n"); }
    #[test] fn test_str_short_circuiting_1() { run_str("if true and print('yes') { print('no') }", "yes\n"); }
    #[test] fn test_str_short_circuiting_2() { run_str("if false and print('also no') { print('no') }", ""); }
    #[test] fn test_str_short_circuiting_3() { run_str("if true and (print('yes') or true) { print('also yes') }", "yes\nalso yes\n"); }
    #[test] fn test_str_short_circuiting_4() { run_str("if false or print('yes') { print('no') }", "yes\n"); }
    #[test] fn test_str_short_circuiting_5() { run_str("if true or print('no') { print('yes') }", "yes\n"); }
    #[test] fn test_str_partial_func_1() { run_str("'apples and bananas' . replace ('a', 'o') . print", "opples ond bononos\n"); }
    #[test] fn test_str_partial_func_2() { run_str("'apples and bananas' . replace ('a') ('o') . print", "opples ond bononos\n"); }
    #[test] fn test_str_partial_func_3() { run_str("print('apples and bananas' . replace ('a') ('o'))", "opples ond bononos\n"); }
    #[test] fn test_str_partial_func_4() { run_str("let x = replace ('a', 'o') ; 'apples and bananas' . x . print", "opples ond bononos\n"); }
    #[test] fn test_str_partial_func_5() { run_str("let x = replace ('a', 'o') ; print(x('apples and bananas'))", "opples ond bononos\n"); }
    #[test] fn test_str_partial_func_6() { run_str("('o' . replace('a')) ('apples and bananas') . print", "opples ond bononos\n"); }


    #[test] fn test_aoc_2022_01_01() { run("aoc_2022_01_01"); }
    #[test] fn test_fibonacci() { run("fibonacci"); }

    fn run_str(code: &'static str, expected: &'static str) {
        let text: &String = &String::from(code);
        let source: &String = &String::from("<test>");
        let compile= compiler::compile(source, text).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let mut vm = VirtualMachine::new(compile, &b""[..], &mut buf);

        let result: Result<(), RuntimeError> = vm.run();
        assert!(vm.stack.is_empty() || result.is_err());

        let mut output: String = String::from_utf8(buf).unwrap();

        match result {
            Ok(_) => {},
            Err(err) => output.push_str(ErrorReporter::new(text, source).format_runtime_error(&err).as_str()),
        }

        assert_eq!(expected, output.as_str());
    }

    fn run(path: &'static str) {
        let root: &String = &trace::test::get_test_resource_path("compiler", path);
        let text: &String = &trace::test::get_test_resource_src(&root);

        let compile= compiler::compile(root, text).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        let mut vm = VirtualMachine::new(compile, &b""[..], &mut buf);

        let result: Result<(), RuntimeError> = vm.run();
        assert!(vm.stack.is_empty());
        assert!(result.is_ok());

        let output: String = String::from_utf8(buf).unwrap();

        trace::test::compare_test_resource_content(&root, output.split("\n").map(|s| String::from(s)).collect::<Vec<String>>());
    }
}
