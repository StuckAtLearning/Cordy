use crate::stdlib::StdBinding;
use crate::vm::opcode::Opcode;
use crate::vm::value::Value;

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct RuntimeError {
    pub error: RuntimeErrorType,
    pub lineno: u16,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum RuntimeErrorType {
    ValueIsNotFunctionEvaluable(Value),
    BindingIsNotFunctionEvaluable(StdBinding),

    IncorrectNumberOfArguments(StdBinding, u8, u8),
    IncorrectNumberOfArgumentsVariadicAtLeastOne(StdBinding),
    IndexOutOfBounds(i64, usize),
    SliceStepZero,

    TypeErrorUnaryOp(Opcode, Value),
    TypeErrorBinaryOp(Opcode, Value, Value),
    TypeErrorBinaryIs(Value, Value),
    TypeErrorCannotConvertToInt(Value),
    TypeErrorCannotCompare(Value, Value),
    TypeErrorCannotSlice(Value),
    TypeErrorSliceArgMustBeInt(&'static str, Value),
    TypeErrorArgMustBeInt(Value),
    TypeErrorArgMustBeIterable(Value),
    TypeErrorFunc1(&'static str, Value),
    TypeErrorFunc2(&'static str, Value, Value),
    TypeErrorFunc3(&'static str, Value, Value, Value),
}
