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

mod fingerprints;
pub mod instruction_set;
mod instructions;
pub mod ip;
pub mod motion;

use std::io;
use std::io::{Read, Write};

use self::instruction_set::exec_instruction;
use super::fungespace::{FungeSpace, FungeValue, SrcIO};

pub use self::instruction_set::{InstructionMode, InstructionResult, InstructionSet};
pub use self::ip::InstructionPointer;
pub use self::motion::MotionCmds;
pub use fingerprints::{all_fingerprints, safe_fingerprints, string_to_fingerprint};

/// Possible results of calling [Interpreter::run]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramResult {
    /// Program finished with the indicated code
    Done(i32),
    /// Catastrophic failure
    Panic,
    /// Program is paused (only returned if using [RunMode::Step])
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IOMode {
    Text,
    Binary,
}

/// Execution mode as indicated by the sysinfo (`y`) instruction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecMode {
    Disabled,
    System,
    SpecificShell,
    SameShell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// Run program to the end
    Run,
    /// Execute a single tick (for all IPs)
    Step,
}

/// State of an rfunge interpreter
pub struct Interpreter<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    /// Currently active IPs
    pub ips: Vec<InstructionPointer<Idx, Space, Env>>,
    /// Funge-space
    pub space: Space,
    /// User-supplied environment permitting access to the outside world
    pub env: Env,
}

/// An interpreter environment provides things like IO and will be implemented
/// differently depending on whether the interpreter is running from the command
/// line, in a web browser, as part of the test suite, etc.
pub trait InterpreterEnv {
    /// Are we using text or binary mode?
    fn get_iomode(&self) -> IOMode;
    /// Should sysinfo (`y`) say that IO is buffered?
    fn is_io_buffered(&self) -> bool;
    /// stdout or equivalent
    fn output_writer(&mut self) -> &mut dyn Write;
    /// stdin or equivalent
    fn input_reader(&mut self) -> &mut dyn Read;
    /// Method called on warnings like "unknown instruction"
    fn warn(&mut self, msg: &str);
    /// What handprint should sysinfo (`y`) name? Default: 0x52464e47
    fn handprint(&self) -> i32 {
        0x52464e47 // RFNG
    }
    /// Is `i` available? (see also: [InterpreterEnv::read_file])
    fn have_file_input(&self) -> bool {
        false
    }
    /// Is `o` available? (see also: [InterpreterEnv::write_file])
    fn have_file_output(&self) -> bool {
        false
    }
    /// Is `=` available, and how does [InterpreterEnv::execute_command] act
    /// (in the terms defined for sysinfo (`y`))?
    fn have_execute(&self) -> ExecMode {
        ExecMode::Disabled
    }
    /// Get the contents of a named file.
    fn read_file(&mut self, _filename: &str) -> io::Result<Vec<u8>> {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
    /// Write data to a named file.
    fn write_file(&mut self, _filename: &str, _content: &[u8]) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
    /// Execute a command and return the exit status
    fn execute_command(&mut self, _command: &str) -> i32 {
        -1
    }
    /// Get the environment variables to pass to the program
    fn env_vars(&mut self) -> Vec<(String, String)> {
        Vec::new()
    }
    /// Get the command line arguments to pass to the program (the first element
    /// should be the name of the script)
    fn argv(&mut self) -> Vec<String> {
        Vec::new()
    }
    /// Is a given fingerprint available? (See also: [all_fingerprints],
    /// [safe_fingerprints])
    fn is_fingerprint_enabled(&self, _fpr: i32) -> bool {
        false
    }
}

impl<Idx, Space, Env> Interpreter<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    pub fn run(&mut self, mode: RunMode) -> ProgramResult {
        let mut stopped_ips = Vec::new();
        let mut new_ips = Vec::new();
        let mut location_log = Vec::new();

        loop {
            for ip_idx in 0..self.ips.len() {
                let mut go_again = true;
                location_log.truncate(0);
                while go_again {
                    let ip = &mut self.ips[ip_idx];
                    let instruction = if ip.must_advance {
                        let (new_loc, new_val) = self.space.move_by(ip.location, ip.delta);
                        // Check that this loop is not infinite
                        if location_log.iter().any(|l| *l == new_loc) {
                            return ProgramResult::Panic;
                        } else {
                            location_log.push(new_loc);
                        }
                        ip.location = new_loc;
                        ip.must_advance = false;
                        *new_val
                    } else {
                        self.space[ip.location]
                    };
                    go_again = false;
                    match exec_instruction(instruction, ip, &mut self.space, &mut self.env) {
                        InstructionResult::Continue => {
                            ip.must_advance = true;
                        }
                        InstructionResult::Skip => {
                            ip.must_advance = true;
                            go_again = true;
                        }
                        InstructionResult::StayPut => (),
                        InstructionResult::Stop => {
                            stopped_ips.push(ip_idx);
                        }
                        InstructionResult::Exit(returncode) => {
                            return ProgramResult::Done(returncode);
                        }
                        InstructionResult::Panic => {
                            return ProgramResult::Panic;
                        }
                        InstructionResult::Fork => {
                            // Find an ID for the new IP
                            let new_id = self.ips.iter().map(|ip| ip.id).max().unwrap() + 1.into();
                            let ip = &mut self.ips[ip_idx]; // re-borrow
                                                            // Create the IP
                            let mut new_ip = ip.clone();
                            new_ip.id = new_id;
                            new_ip.delta = ip.delta * (-1).into();
                            let (new_loc, _) = self.space.move_by(ip.location, new_ip.delta);
                            new_ip.location = new_loc;
                            new_ips.push((ip_idx, new_ip));
                            // Move the parent along
                            ip.must_advance = true;
                        }
                    }
                }
            }

            // handle forks
            for (ip_idx, new_ip) in new_ips.drain(0..) {
                self.ips.insert(ip_idx, new_ip);
                // Fix ip indices in stopped_ips
                for idx in stopped_ips.iter_mut() {
                    if *idx >= ip_idx {
                        *idx += 1;
                    }
                }
            }

            // handle stops
            for idx in stopped_ips.drain(0..).rev() {
                self.ips.remove(idx);
            }

            if self.ips.is_empty() {
                return ProgramResult::Done(0);
            }

            if mode == RunMode::Step {
                return ProgramResult::Paused;
            }
        }
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
