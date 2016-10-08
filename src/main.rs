use std::{env, io};
use std::collections::HashMap;
use std::num::Wrapping;
use std::fs::File;
use std::io::Read;

struct Program {
    code: Vec<u8>,
    loop_links: HashMap<usize, usize>,
}

#[derive(Debug)]
enum ProgramError {
    LoopBeginningWithoutEnd,
    LoopEndWithoutBeginning,
}

impl Program {
    fn is_valid_bchar(bchar: u8) -> bool {
        match bchar {
            b'>' | b'<' | b'+' | b'-' | b'.' | b',' | b'[' | b']' => true,
            _ => false,
        }
    }

    fn new(input_code: &Vec<u8>) -> Result<Self, ProgramError> {
        let mut program = Program { code: input_code.clone(), loop_links: HashMap::new() };
        program.code.retain(|&bchar| Program::is_valid_bchar(bchar)); // strip out non-code characters

        // resolve loop links
        let mut unfinished_loop_links = Vec::new();
        for (i, &bchar) in program.code.iter().enumerate() {
            if bchar == b'[' {
                unfinished_loop_links.push(i);
            } else if bchar == b']' {
                if unfinished_loop_links.len() > 0 {
                    let loop_beginning = unfinished_loop_links.pop().unwrap();
                    program.loop_links.insert(loop_beginning, i);
                    program.loop_links.insert(i, loop_beginning);
                } else {
                    return Err(ProgramError::LoopEndWithoutBeginning);
                }
            }
        }

        if unfinished_loop_links.len() > 0 {
            return Err(ProgramError::LoopBeginningWithoutEnd);
        }
        
        Ok(program)
    }
}

const PROGRAM_MEMORY: usize = 30000;

struct ProgramRuntime {
    pc: Wrapping<usize>,
    mem: [Wrapping<u8>; PROGRAM_MEMORY],
    mem_ptr: Wrapping<usize>,
}

#[derive(Debug)]
enum ProgramRuntimeError {
    ReadAccessViolation,
    WriteAccessViolation,
}

#[derive(Debug)]
enum ProgramRuntimeStatus {
    RanInstructionAtPC(usize),
    EndOfProgram,
}

impl ProgramRuntime {
    fn new() -> Self {
        ProgramRuntime {
            pc: Wrapping(0),
            mem: [Wrapping(0); PROGRAM_MEMORY],
            mem_ptr: Wrapping(0),
        }
    }

    fn read_mem(&self, loc: usize) -> Result<u8, ProgramRuntimeError> {
        if loc < self.mem.len() {
            Ok(self.mem[loc].0)
        } else {
            Err(ProgramRuntimeError::ReadAccessViolation)
        }
    }

    fn read_mem_at_ptr(&self) -> Result<u8, ProgramRuntimeError> {
        self.read_mem(self.mem_ptr.0)
    }

    fn write_mem(&mut self, loc: usize, val: u8) -> Result<(), ProgramRuntimeError> {
        if loc < self.mem.len() {
            self.mem[loc] = Wrapping(val);
            Ok(())
        } else {
            Err(ProgramRuntimeError::WriteAccessViolation)
        }
    }
    
    fn write_mem_at_ptr(&mut self, val: u8) -> Result<(), ProgramRuntimeError> {
        let loc = self.mem_ptr.0;
        self.write_mem(loc, val)
    }

    fn inc_mem_at_ptr(&mut self) -> Result<u8, ProgramRuntimeError> {
        let loc = self.mem_ptr.0;
        if loc < self.mem.len() {
            self.mem[loc] += Wrapping(1);
            Ok(self.mem[loc].0)
        } else {
            Err(ProgramRuntimeError::WriteAccessViolation)
        }
    }

    fn dec_mem_at_ptr(&mut self) -> Result<u8, ProgramRuntimeError> {
        let loc = self.mem_ptr.0;
        if loc < self.mem.len() {
            self.mem[loc] -= Wrapping(1);
            Ok(self.mem[loc].0)
        } else {
            Err(ProgramRuntimeError::WriteAccessViolation)
        }
    }

    fn step(&mut self, program: &Program) -> Result<ProgramRuntimeStatus, ProgramRuntimeError> {
        let mut next_pc = self.pc + Wrapping(1);
        let pc = self.pc.0;

        if pc >= program.code.len() {
            return Ok(ProgramRuntimeStatus::EndOfProgram);
        }

        match program.code[pc] {
            b'>' => self.mem_ptr += Wrapping(1),
            b'<' => self.mem_ptr -= Wrapping(1),
            b'+' => { try!(self.inc_mem_at_ptr()); },
            b'-' => { try!(self.dec_mem_at_ptr()); },
            b'.' => print!("{}", try!(self.read_mem_at_ptr()) as char),
            b',' => {
                // read byte from stdin and store at ptr
                let mut read_buf: [u8; 1] = [0];
                io::stdin().read_exact(&mut read_buf).expect("Failed to read from stdin");
                try!(self.write_mem_at_ptr(read_buf[0]));
            },
            b'[' => {
                // jump past matching ] if mem at ptr is 0
                if try!(self.read_mem_at_ptr()) == 0 {
                    next_pc = Wrapping(*program.loop_links.get(&pc).unwrap()) + Wrapping(1);
                }
            },
            b']' => {
                // jump back past matching [ if mem at ptr is NOT 0
                if try!(self.read_mem_at_ptr()) != 0 {
                    next_pc = Wrapping(*program.loop_links.get(&pc).unwrap()) + Wrapping(1);
                }
            },
            bchar => debug_assert!(!Program::is_valid_bchar(bchar), "Non-code char wasn't stripped!"),
        }

        self.pc = next_pc;
        Ok(ProgramRuntimeStatus::RanInstructionAtPC(pc))
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // display help if no file path input or help switch. run otherwise
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" || args[1] == "-?" {
        println!("rustedbrain - A Brainf*ck language interpreter written in Rust.");
        println!("Written by Sean Dewar (seandewar @ github). Version {}.",
                 option_env!("CARGO_PKG_VERSION").unwrap_or("[UNKNOWN]"));
        println!("");
        println!("Usage:");
        println!("  rustedbrain <file-path>           Run the script at <file-path>");
        println!("  rustedbrain | -h | -? | --help    Display this help message");
    } else {
        let mut file_data = Vec::new();
        let mut file = File::open(&args[1]).expect("Failed to open file");
        file.read_to_end(&mut file_data).expect("Failed to read file data");

        let program = Program::new(&file_data).expect("Failed to load program");
        let mut program_runtime = ProgramRuntime::new();
        loop {
            let runtime_status = program_runtime.step(&program).expect("Program runtime execution error");
            if let ProgramRuntimeStatus::EndOfProgram = runtime_status {
                break;
            }
        }
    }
}
