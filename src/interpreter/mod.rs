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

pub mod fingerprints;
pub mod instruction_set;
mod instructions;
pub mod ip;
pub mod motion;

use std::any::Any;
use std::io;
use std::marker::Unpin;

use futures_lite::future::block_on;
use futures_lite::io::{AsyncRead, AsyncWrite};

use self::instruction_set::exec_instruction;
use self::ip::CreateInstructionPointer;
use super::fungespace::{FungeSpace, FungeValue, SrcIO};

pub use self::instruction_set::{InstructionContext, InstructionMode, InstructionResult};
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
    /// Run up to a certain number of instructions
    Limited(u32),
}

pub trait Funge {
    type Idx: MotionCmds<Self::Space, Self::Env> + SrcIO<Self::Space> + 'static;
    type Space: FungeSpace<Self::Idx, Output = Self::Value> + 'static;
    type Value: FungeValue + 'static;
    type Env: InterpreterEnv + 'static;
}

/// State of an rfunge interpreter
pub struct Interpreter<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    /// Currently active IPs
    pub ips: Vec<Option<InstructionPointer<Self>>>,
    /// Funge-space
    pub space: Option<Space>,
    /// User-supplied environment permitting access to the outside world
    pub env: Option<Env>,
}

impl<Idx, Space, Env> Funge for Interpreter<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    type Idx = Idx;
    type Space = Space;
    type Value = Space::Output;
    type Env = Env;
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
    fn output_writer(&mut self) -> &mut (dyn AsyncWrite + Unpin);
    /// stdin or equivalent
    fn input_reader(&mut self) -> &mut (dyn AsyncRead + Unpin);
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
    /// Get the support library for a particular fingerprint that needs
    /// environment support, if available.
    fn fingerprint_support_library(&mut self, _fpr: i32) -> Option<&mut dyn Any> {
        None
    }
}

impl<Idx, Space, Env> Interpreter<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    pub async fn run_async(&mut self, mode: RunMode) -> ProgramResult {
        let mut stopped_ips = Vec::new();
        let mut new_ips = Vec::new();
        let mut location_log = Vec::new();
        let mut counter: u32 = 0;

        loop {
            for ip_idx in 0..self.ips.len() {
                let mut go_again = true;
                location_log.truncate(0);
                while go_again {
                    let ip = self.ips[ip_idx].as_ref().unwrap();
                    let (new_loc, new_val) =
                        self.space.as_mut().unwrap().move_by(ip.location, ip.delta);
                    let instruction = *new_val;
                    // Check that this loop is not infinite
                    if location_log.iter().any(|l| *l == new_loc) {
                        return ProgramResult::Panic;
                    } else {
                        location_log.push(new_loc);
                    }
                    // Move everything to an instruction context
                    let mut ctx = InstructionContext {
                        ip: self.ips[ip_idx].take().unwrap(),
                        space: self.space.take().unwrap(),
                        env: self.env.take().unwrap(),
                    };
                    ctx.ip.location = new_loc;

                    go_again = false;
                    // Hand context over to exec_instruction
                    let (ctx, result) = exec_instruction(instruction, ctx).await;
                    // Move everything from `ctx` back to `self`
                    self.ips[ip_idx].replace(ctx.ip);
                    self.space.replace(ctx.space);
                    self.env.replace(ctx.env);
                    // Continue
                    match result {
                        InstructionResult::Continue => {}
                        InstructionResult::Skip => {
                            go_again = true;
                        }
                        InstructionResult::Stop => {
                            stopped_ips.push(ip_idx);
                        }
                        InstructionResult::Exit(returncode) => {
                            return ProgramResult::Done(returncode);
                        }
                        InstructionResult::Panic => {
                            return ProgramResult::Panic;
                        }
                        InstructionResult::Fork(n_forks) => {
                            // Find an ID for the new IP
                            let mut new_id = self
                                .ips
                                .iter()
                                .map(|ip| ip.as_ref().unwrap().id)
                                .max()
                                .unwrap()
                                + 1.into();
                            for _ in 0..n_forks {
                                let ip = &mut self.ips[ip_idx].as_mut().unwrap(); // borrow
                                let mut new_ip = ip.clone(); // Create the IP
                                new_ip.id = new_id;
                                new_id += 1.into();
                                new_ip.delta = ip.delta * (-1).into();
                                new_ips.push((ip_idx, new_ip));
                            }
                        }
                    }
                }
            }

            // handle forks
            for (ip_idx, new_ip) in new_ips.drain(0..).rev() {
                self.ips.insert(ip_idx, Some(new_ip));
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

            match mode {
                RunMode::Run => (),
                RunMode::Step => return ProgramResult::Paused,
                RunMode::Limited(max_instructions) => {
                    counter += 1;
                    if counter >= max_instructions {
                        return ProgramResult::Paused;
                    }
                }
            }
        }
    }

    pub fn run(&mut self, mode: RunMode) -> ProgramResult {
        block_on(self.run_async(mode))
    }
}

impl<Idx, Space, Env> Interpreter<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + CreateInstructionPointer<Space, Env> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    pub fn new(space: Space, env: Env) -> Self {
        Self {
            ips: vec![Some(InstructionPointer::<Self>::new())],
            space: Some(space),
            env: Some(env),
        }
    }
}

#[cfg(test)]
mod tests {
    use async_std::io::{Empty, Sink};

    use super::*;
    use crate::fungespace::{BefungeVec, PagedFungeSpace};

    pub struct NoEnv {
        input: Empty,
        outout: Sink,
    }

    impl InterpreterEnv for NoEnv {
        fn get_iomode(&self) -> IOMode {
            IOMode::Text
        }
        fn is_io_buffered(&self) -> bool {
            true
        }
        fn output_writer(&mut self) -> &mut (dyn AsyncWrite + Unpin) {
            &mut self.outout
        }
        fn input_reader(&mut self) -> &mut (dyn AsyncRead + Unpin) {
            &mut self.input
        }
        fn warn(&mut self, _msg: &str) {}
    }

    pub struct TestFunge {}

    impl Funge for TestFunge {
        type Idx = BefungeVec<i64>;
        type Space = PagedFungeSpace<BefungeVec<i64>, i64>;
        type Value = i64;
        type Env = NoEnv;
    }
}
