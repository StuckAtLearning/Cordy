use std::rc::Rc;
use crate::reporting::{AsError, AsErrorWithContext, Location, Locations, SourceView};

use crate::stdlib::NativeFunction;
use crate::vm::CallFrame;
use crate::vm::operator::{BinaryOp, UnaryOp};
use crate::vm::value::{FunctionImpl, Value};


#[derive(Debug, Clone)]
pub enum RuntimeError {
    RuntimeExit,
    RuntimeYield,

    RuntimeCompilationError(Vec<String>),

    ValueIsNotFunctionEvaluable(Value),

    IncorrectNumberOfFunctionArguments(FunctionImpl, u8),
    IncorrectNumberOfArguments(NativeFunction, u8, u8),
    IncorrectNumberOfArgumentsVariadicAtLeastOne(NativeFunction),

    ValueErrorIndexOutOfBounds(i64, usize),
    ValueErrorStepCannotBeZero,
    ValueErrorVariableNotDeclaredYet(String),
    ValueErrorValueMustBeNonNegative(i64),
    ValueErrorValueMustBePositive(i64),
    ValueErrorValueMustBeNonZero,
    ValueErrorValueMustBeNonEmpty,
    ValueErrorCannotUnpackLengthMustBeGreaterThan(usize, usize, Value), // expected, actual
    ValueErrorCannotUnpackLengthMustBeEqual(usize, usize, Value), // expected, actual
    ValueErrorCannotCollectIntoDict(Value),
    ValueErrorKeyNotPresent(Value),
    ValueErrorInvalidCharacterOrdinal(i64),
    ValueErrorInvalidFormatCharacter(Option<char>),
    ValueErrorNotAllArgumentsUsedInStringFormatting(Value),
    ValueErrorMissingRequiredArgumentInStringFormatting,
    ValueErrorEvalListMustHaveUnitLength(usize),

    TypeErrorUnaryOp(UnaryOp, Value),
    TypeErrorBinaryOp(BinaryOp, Value, Value),
    TypeErrorBinaryIs(Value, Value),
    TypeErrorCannotConvertToInt(Value),

    TypeErrorArgMustBeInt(Value),
    TypeErrorArgMustBeStr(Value),
    TypeErrorArgMustBeChar(Value),
    TypeErrorArgMustBeIterable(Value),
    TypeErrorArgMustBeIndexable(Value),
    TypeErrorArgMustBeSliceable(Value),
    TypeErrorArgMustBeDict(Value),
    TypeErrorArgMustBeFunction(Value),
    TypeErrorArgMustBeCmpOrKeyFunction(Value),
}

impl RuntimeError {
    #[cold]
    pub fn err<T>(self: Self) -> Result<T, Box<RuntimeError>> {
        Err(Box::new(self))
    }

    pub fn with_stacktrace(self: Self, ip: usize, call_stack: &Vec<CallFrame>, functions: &Vec<Rc<FunctionImpl>>, locations: &Locations) -> DetailRuntimeError {
        const REPEAT_LIMIT: usize = 0;

        // Top level stack frame refers to the code being executed
        let target: Location = locations[ip];
        let mut stack: Vec<StackFrame> = Vec::new();
        let mut prev_ip: usize = ip;
        let mut prev_frame: Option<(usize, usize)> = None;
        let mut prev_count: usize = 0;

        for frame in call_stack.iter().rev() {
            if frame.return_ip > 0 {
                // Each frame from the call stack refers to the owning function of the previous frame
                let frame_ip: usize = frame.return_ip - 1;

                if prev_frame == Some((frame_ip, prev_ip)) {
                    prev_count += 1;
                } else {
                    if prev_count > REPEAT_LIMIT {
                        // Push a 'repeat' element
                        stack.push(StackFrame::Repeat(prev_count - REPEAT_LIMIT))
                    }
                    prev_count = 0;
                }

                if prev_count <= REPEAT_LIMIT {
                    stack.push(StackFrame::Simple(frame_ip, locations[frame_ip], find_owning_function(prev_ip, functions)));
                }

                prev_frame = Some((frame_ip, prev_ip));
                prev_ip = frame_ip;
            }
        }

        if prev_count > REPEAT_LIMIT {
            stack.push(StackFrame::Repeat(prev_count - REPEAT_LIMIT))
        }

        DetailRuntimeError { error: self, target, stack }
    }
}

/// A `RuntimeError` with a filled-in stack trace, and source location which caused the error.
#[derive(Debug)]
pub struct DetailRuntimeError {
    error: RuntimeError,
    target: Location,

    /// The stack trace elements, including a location (typically of the function call), and the function name itself
    stack: Vec<StackFrame>,
}

#[derive(Debug)]
enum StackFrame {
    Simple(usize, Location, String),
    Repeat(usize),
}

impl AsError for DetailRuntimeError {
    fn as_error(self: &Self) -> String {
        self.error.as_error()
    }
}

impl AsErrorWithContext for DetailRuntimeError {
    fn location(self: &Self) -> Location {
        self.target
    }

    fn add_stack_trace_elements(self: &Self, view: &SourceView, text: &mut String) {
        for frame in &self.stack {
            text.push_str(match frame {
                StackFrame::Simple(_, loc, site) => format!("  at: `{}` (line {})\n", site, view.lineno(*loc) + 1),
                StackFrame::Repeat(n) => format!("  ... above line repeated {} more time(s) ...\n", n),
            }.as_str());
        }
    }
}


/// The owning function for a given IP can be defined as the closest function which encloses the desired instruction
/// We annotate both head and tail of `FunctionImpl` to make this search easy
fn find_owning_function(ip: usize, functions: &Vec<Rc<FunctionImpl>>) -> String {
    functions.iter()
        .filter(|f| f.head <= ip && ip <= f.tail)
        .min_by_key(|f| f.tail - f.head)
        .map(|f| f.as_str())
        .unwrap_or_else(|| String::from("<script>"))
}
