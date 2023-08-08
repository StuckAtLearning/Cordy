use std::{fs, io};
use std::io::Write;
use rustyline::{DefaultEditor, Editor};
use rustyline::error::ReadlineError;

use cordy_sys::{compiler, repl};
use cordy_sys::compiler::CompileResult;
use cordy_sys::repl::Reader;
use cordy_sys::SourceView;
use cordy_sys::vm::{ExitType, VirtualMachine};


fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut options: Options = match parse_args(args) {
        Some(args) => args,
        None => return
    };
    let result = match options.file.take() {
        Some(name) => run_main(name, options),
        None => run_repl()
    };
    match result {
        Ok(()) => {},
        Err(e) => eprintln!("{}", e)
    }
}

fn run_main(name: String, options: Options) -> Result<(), String> {
    let text: String = fs::read_to_string(&name).map_err(|_| format!("Unable to read file '{}'", name))?;
    let view: SourceView = SourceView::new(name, text);
    let compiled: CompileResult = compiler::compile(options.optimize, &view).map_err(|e| e.join("\n"))?;

    match options.mode {
        Mode::Disassembly => {
            for line in compiled.disassemble(&view, !options.no_line_numbers) {
                println!("{}", line);
            }
            Ok(())
        },
        Mode::Default => run_vm(compiled, options.args, view)
    }
}

fn parse_args(args: Vec<String>) -> Option<Options> {
    let mut iter = args.into_iter();
    let mut options: Options = Options {
        file: None,
        args: Vec::new(),
        mode: Mode::Default,
        optimize: false,
        no_line_numbers: false
    };

    if iter.next().is_none() {
        panic!("Unexpected first argument");
    }

    for arg in iter.by_ref() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return None;
            },
            "-d" | "--disassembly" => options.mode.set(Mode::Disassembly).ok()?,
            "-o" | "--optimize" => options.optimize = true,
            "--no-line-numbers" => options.no_line_numbers = true,
            a => {
                options.file = Some(String::from(a));
                break
            },
        }
    }

    options.args.extend(iter);
    Some(options)
}

fn run_vm(compiled: CompileResult, program_args: Vec<String>, view: SourceView) -> Result<(), String> {

    let stdin = io::stdin().lock();
    let stdout = io::stdout();
    let mut vm = VirtualMachine::new(compiled, view, stdin, stdout, program_args);

    match vm.run_until_completion() {
        ExitType::Error(error) => Err(vm.view().format(&error)),
        _ => Ok(())
    }
}

pub fn run_repl() -> Result<(), String> {
    println!("Welcome to Cordy! (exit with 'exit' or Ctrl-C)");
    repl::run(EditorRepl { editor: Editor::new().unwrap() }, io::stdout(), false)
}

struct EditorRepl {
    editor: DefaultEditor
}

impl Reader for EditorRepl {
    fn read(&mut self, prompt: &'static str) -> Option<Result<String, String>> {
        io::stdout().flush().unwrap();
        match self.editor.readline(prompt) {
            Ok(line) => {
                self.editor.add_history_entry(line.as_str()).unwrap();
                Some(Ok(line))
            },
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => None,
            Err(e) => Some(Err(format!("Error: {}", e))),
        }
    }
}



fn print_help() {
    println!("cordy [options] <file> [program arguments...]");
    println!("When invoked with no arguments, this will open a REPL for the Cordy language (exit with 'exit' or Ctrl-C)");
    println!("Options:");
    println!("  -h --help         : Show this message and then exit.");
    println!("  -d --disassembly  : Dump the disassembly view. Does nothing in REPL mode.");
    println!("  --no-line-numbers : In disassembly view, omits the leading '0001' style line numbers");
    println!("  -o --optimize     : Enables compiler optimizations.");
}

struct Options {
    file: Option<String>,
    args: Vec<String>,
    mode: Mode,
    optimize: bool,
    no_line_numbers: bool,
}

#[derive(Eq, PartialEq)]
enum Mode { Default, Disassembly }

impl Mode {
    fn set(&mut self, new: Mode) -> Result<(), String> {
        if *self != Mode::Default {
            Err(String::from("Must only specify one of --disassembly"))
        } else {
            *self = new;
            Ok(())
        }
    }
}
