use crate::core::NativeFunction;
use crate::vm::operator::{BinaryOp, UnaryOp};
use crate::vm::value::LiteralType;
use crate::util::OffsetAdd;

use Opcode::{*};
use crate::compiler::Fields;
use crate::vm::Value;

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Opcode {

    Noop,

    /// The parameter is an offset value, based on the IP *after* the instruction is executed. So an instruction:
    ///
    /// `005: Jump(-2)`
    ///
    /// will jump to 005 + 1 (since we finish executing this instruction) - 2 = 004
    ///
    /// `Jump(-1)` is a no-op.
    JumpIfFalse(i32),
    JumpIfFalsePop(i32),
    JumpIfTrue(i32),
    JumpIfTruePop(i32),
    Jump(i32),

    Return,

    // Stack Operations
    Pop,
    PopN(u32),
    Dup,
    Swap,

    PushLocal(u32),
    StoreLocal(u32),
    PushGlobal(u32),
    StoreGlobal(u32),

    PushUpValue(u32), // index
    StoreUpValue(u32), // index

    StoreArray,

    // Increments the count of currently declared global variables. This is checked on every `CheckGlobalCount` to verify that no global is referenced before it is initialized
    // Due to late binding allowed in the parser, we cannot ensure this does not happen at runtime, so it needs to be checked.
    IncGlobalCount,

    // Create a closure from a function (with no upvalues yet)
    Closure,

    // Take a closure, and add the respective local or upvalue to the closure
    CloseLocal(u32),
    CloseUpValue(u32),

    /// Lifts an UpValue from a stack slot (offset by the frame pointer) to the heap
    /// It does so by boxing it into a `Rc<Cell<Value>>`, stored on the closure's `environment` array. Each closure references the same `UpValue`, and hence will see all mutations.
    /// Takes a local index of an upvalue to lift.
    LiftUpValue(u32),

    /// Converts the top of the stack to an `Value::Iter()`.
    InitIterable,

    /// Expects the top of the stack to contain a `Value::Iter()`. Tests if this has reached the end of the iterable.
    /// If yes, it will push `false`, and do nothing else.
    /// If no, it will push the next value in the iterable, followed by `true`.
    /// This is intended to be followed by a `JumpIfFalsePop(end of loop)`
    TestIterable,

    /// Pushes constant values `Nil`, `True` and `False
    Nil,
    True,
    False,

    /// Pushes a constant from `vm.constants[index]`.
    /// This is used for  all integer, complex, string, function and struct types.
    Constant(u32),
    NativeFunction(NativeFunction),

    /// Pushes a new, empty `Literal` onto the literal stack, of a given literal sequence type (`list`, `set`, `dict`, or `vector`), and size hint `u32`.
    /// Initialization of a literal will always start with a `LiteralBegin`, followed by one more more `LiteralAcc` and `LiteralUnroll` opcodes, then `LiteralEnd`
    LiteralBegin(LiteralType, u32),

    /// Pops `u32` entries from the stack and inserts them, in order, to the top of the literal stack.
    LiteralAcc(u32),

    /// Pops the top of the stack, and unrolls it and inserts each element, in order, to the top of the literal stack.
    /// This is different from doing a `OpUnroll` as we have special handling for accumulating and unrolling `dict`s
    LiteralUnroll,

    // Pops the top of the literal stack, and pushes it as a value onto the stack.
    LiteralEnd,

    /// Checks that the top of the stack is an iterable with length > the provided value
    CheckLengthGreaterThan(u32),
    /// Checks that the top of the stack is an iterable with length = the provided value
    CheckLengthEqualTo(u32),

    /// Opcode for function evaluation (either with `()` or with `.`). The `u8` parameter is the number of arguments to the function.
    ///
    /// - The stack must be setup with the function to be called (which must be a callable type, i.e. return `true` to `value.is_function()`, followed by `n` arguments.
    /// - A call frame will be pushed, and the `frame_pointer()` of the VM will point to the first local of the new frame. The function itself can be accessed via `stack[frame_pointer() - 1]`.
    /// - Upon reaching a `Return` opcode, everything above and including `frame_pointer() - 1` will be popped off the stack, and the return value will be pushed onto the stack (replacing the spot where the function was).
    ///
    /// Implementation Note: In order to implement function composition (the `.` operator), this is preceded with a `Swap` opcode to reverse the order of the argument and function to be called.
    OpFuncEval(u32),
    OpFuncEvalUnrolled(u32),

    /// Unrolls an iterable on the stack. Used in combination with `OpFuncEvalUnrolled` to call functions with `...`. Also can be used with list, vector, and dict initializations.
    /// The argument is if this unroll is the first one we've seen in the *current function invocation*. If so, it pushes a new counter onto the stack.
    OpUnroll(bool),

    /// Takes a stack of `[index, list, ...]`, pops the top two elements, and pushes `list[index]`
    OpIndex,
    /// Takes a stack of `[index, list, ...]`, and pushes `list[index]` (does not pop any values)
    OpIndexPeek,

    OpSlice,
    OpSliceWithStep,

    GetField(u32),
    GetFieldPeek(u32),
    GetFieldFunction(u32),
    SetField(u32),

    Unary(UnaryOp),
    Binary(BinaryOp),

    /// Creates a `slice` object, which is used to perform slice operations
    Slice,
    SliceWithStep,

    // Special
    Exit,
    Yield,
    AssertFailed,
}

impl Opcode {

    pub fn disassembly<I : Iterator<Item=String>>(self: &Opcode, ip: usize, locals: &mut I, fields: &Fields, constants: &Vec<Value>) -> String {
        match self {
            Constant(id) => {
                let constant = &constants[*id as usize];
                match constant {
                    Value::Nil => String::from("Nil"),
                    Value::Bool(it) => String::from(if *it { "True" } else { "False" }),
                    Value::Function(it) => format!("Function({} -> L[{}, {}])", it.as_str(), it.head, it.tail),
                    Value::StructType(it) => format!("StructType({})", it.as_str()),
                    _ => format!("{}({})", match constant {
                        Value::Int(_) => "Int",
                        Value::Str(_) => "Str",
                        Value::Complex(_) => "Complex",
                        _ => panic!("Not a constant: {:?}", constant),
                    }, constant.to_repr_str())
                }
            },
            CheckLengthEqualTo(len) | CheckLengthGreaterThan(len) => format!("{:?} -> {}", self, len),
            PushGlobal(_) | StoreGlobal(_) | PushLocal(_) | StoreLocal(_) => match locals.next() {
                Some(local) => format!("{:?} -> {}", self, local),
                None => format!("{:?}", self),
            },
            GetField(fid) | SetField(fid) | GetFieldFunction(fid) => format!("{:?} -> {}", self, fields.get_field_name(*fid)),
            JumpIfFalse(offset) | JumpIfFalsePop(offset) | JumpIfTrue(offset) | JumpIfTruePop(offset) | Jump(offset) => format!("{}({})", match self {
                JumpIfFalse(_) => "JumpIfFalse",
                JumpIfFalsePop(_) => "JumpIfFalsePop",
                JumpIfTrue(_) => "JumpIfTrue",
                JumpIfTruePop(_) => "JumpIfTruePop",
                Jump(_) => "Jump",
                _ => unreachable!()
            }, ip.add_offset(*offset + 1)),
            Binary(op) => format!("{:?}", op),
            Unary(op) => format!("{:?}", op),
            NativeFunction(op) => format!("{:?}", op),
            OpUnroll(_) => format!("Unroll"),
            OpFuncEval(n) => format!("Call({})", n),
            OpFuncEvalUnrolled(n) => format!("Call...({})", n),
            _ => format!("{:?}", self),
        }
    }
}


#[cfg(test)]
mod test {
    use crate::vm::opcode::Opcode;

    #[test] fn test_layout() { assert_eq!(8, std::mem::size_of::<Opcode>()); }
}