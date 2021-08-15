/*
rfunge – a Funge-98 interpreter
Copyright © 2021 Thomas Jollans

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

pub mod instruction_set;
mod instructions;
pub mod ip;
pub mod motion;

use std::io;
use std::io::{Read, Write};

use self::instruction_set::exec_instruction;
pub use self::instruction_set::{InstructionMode, InstructionResult, InstructionSet};
pub use self::ip::InstructionPointer;
pub use self::motion::MotionCmds;
use super::fungespace::{FungeSpace, FungeValue, SrcIO};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramResult {
    Ok,
    Panic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IOMode {
    Text,
    Binary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecMode {
    Disabled,
    System,
    SpecificShell,
    SameShell,
}

pub struct Interpreter<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    pub ips: Vec<InstructionPointer<Idx, Space, Env>>,
    pub space: Space,
    pub env: Env,
}

pub trait InterpreterEnv {
    fn get_iomode(&self) -> IOMode;
    fn is_io_buffered(&self) -> bool;
    fn output_writer(&mut self) -> &mut dyn Write;
    fn input_reader(&mut self) -> &mut dyn Read;
    fn warn(&mut self, msg: &str);
    fn handprint(&self) -> i32 {
        0x52464e47 // RFNG
    }
    fn have_file_input(&self) -> bool {
        false
    }
    fn have_file_output(&self) -> bool {
        false
    }
    fn have_execute(&self) -> ExecMode {
        ExecMode::Disabled
    }
    fn read_file(&mut self, _filename: &str) -> io::Result<Vec<u8>> {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
    fn write_file(&mut self, _filename: &str, _content: &[u8]) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
    fn execute_command(&mut self, _command: &str) -> i32 {
        -1
    }
    fn env_vars(&mut self) -> Vec<(String, String)> {
        Vec::new()
    }
    fn timestamp(&mut self) -> i64 {
        0
    }
    fn argv(&mut self) -> Vec<String> {
        Vec::new()
    }
}

impl<Idx, Space, Env> Interpreter<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    pub fn run(&mut self) -> ProgramResult {
        let ip_idx = self.ips.len() - 1;
        let mut next_instruction = self.space[self.ips[ip_idx].location];

        loop {
            let ip = &mut self.ips[ip_idx];
            let result = exec_instruction(next_instruction, ip, &mut self.space, &mut self.env);

            match result {
                InstructionResult::Continue | InstructionResult::Skip => {
                    // Skip will need special treatment in concurrent funge
                    let (new_loc, new_val) = self.space.move_by(ip.location, ip.delta);
                    ip.location = new_loc;
                    next_instruction = *new_val;
                }
                InstructionResult::StayPut => {
                    next_instruction = self.space[ip.location];
                }
                InstructionResult::Exit => {
                    break;
                }
                InstructionResult::Panic => {
                    return ProgramResult::Panic;
                }
            };
        }

        ProgramResult::Ok
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    pub struct NoEnv {
        input: io::Empty,
        outout: io::Sink,
    }

    // impl NoEnv {
    //     fn new() -> Self { Self { input: io::empty(), outout: io::sink() } }
    // }

    impl InterpreterEnv for NoEnv {
        fn get_iomode(&self) -> IOMode {
            IOMode::Text
        }
        fn is_io_buffered(&self) -> bool {
            true
        }
        fn output_writer(&mut self) -> &mut dyn Write {
            &mut self.outout
        }
        fn input_reader(&mut self) -> &mut dyn Read {
            &mut self.input
        }
        fn warn(&mut self, _msg: &str) {}
    }
}
