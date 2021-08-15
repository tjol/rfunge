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
pub mod ip;

use std::io::{Read, Write};
use std::ops::{Add, Mul};

use self::instruction_set::exec_instruction;
pub use self::instruction_set::{InstructionMode, InstructionResult, InstructionSet};
pub use self::ip::InstructionPointer;
use super::fungespace::index::{bfvec, BefungeVec};
use super::fungespace::{FungeIndex, FungeSpace, FungeValue};

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

pub trait MotionCmds<Space>:
    FungeIndex + Add<Output = Self> + Mul<Space::Output, Output = Self>
where
    Space: FungeSpace<Self>,
    Space::Output: FungeValue,
{
    fn apply_delta(instruction: char, ip: &mut InstructionPointer<Self, Space>) -> bool;
    fn pop_vector(ip: &mut InstructionPointer<Self, Space>) -> Self;
    fn push_vector(ip: &mut InstructionPointer<Self, Space>, v: Self);
}

pub struct Interpreter<Idx, Space, Env>
where
    Idx: MotionCmds<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    pub ips: Vec<InstructionPointer<Idx, Space>>,
    pub space: Space,
    pub env: Env,
}

pub trait InterpreterEnv {
    fn get_iomode(&self) -> IOMode;
    fn output_writer(&mut self) -> &mut dyn Write;
    fn input_reader(&mut self) -> &mut dyn Read;
    fn warn(&mut self, msg: &str);
}

pub struct GenericEnv<Rd, Wr, Wfn>
where
    Rd: Read,
    Wr: Write,
    Wfn: FnMut(&str),
{
    pub io_mode: IOMode,
    pub input: Rd,
    pub output: Wr,
    pub warning_cb: Wfn,
}

impl<Rd, Wr, Wfn> InterpreterEnv for GenericEnv<Rd, Wr, Wfn>
where
    Rd: Read,
    Wr: Write,
    Wfn: FnMut(&str),
{
    fn get_iomode(&self) -> IOMode {
        self.io_mode
    }
    fn output_writer(&mut self) -> &mut dyn Write {
        &mut self.output
    }
    fn input_reader(&mut self) -> &mut dyn Read {
        &mut self.input
    }
    fn warn(&mut self, msg: &str) {
        (self.warning_cb)(msg)
    }
}

impl<Idx, Space, Env> Interpreter<Idx, Space, Env>
where
    Idx: MotionCmds<Space>,
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

// Unefunge implementation of MotionCmds
impl<T, Space> MotionCmds<Space> for T
where
    T: FungeValue,
    Space: FungeSpace<Self, Output = T>,
{
    fn apply_delta(instruction: char, ip: &mut InstructionPointer<Self, Space>) -> bool {
        match instruction {
            '>' => {
                ip.delta = T::from(1);
                true
            }
            '<' => {
                ip.delta = T::from(-1);
                true
            }
            '_' => {
                let p = ip.pop();
                ip.delta = if p == T::zero() {
                    T::from(1)
                } else {
                    T::from(-1)
                };
                true
            }
            _ => false,
        }
    }

    fn pop_vector(ip: &mut InstructionPointer<Self, Space>) -> Self {
        ip.pop()
    }

    fn push_vector(ip: &mut InstructionPointer<Self, Space>, v: Self) {
        ip.push(v);
    }
}

// Befunge implementation of MotionCmds
impl<T, Space> MotionCmds<Space> for BefungeVec<T>
where
    Space: FungeSpace<Self, Output = T>,
    T: FungeValue,
{
    fn apply_delta(instruction: char, ip: &mut InstructionPointer<Self, Space>) -> bool {
        match instruction {
            '>' => {
                ip.delta = bfvec(1, 0);
                true
            }
            '<' => {
                ip.delta = bfvec(-1, 0);
                true
            }
            '^' => {
                ip.delta = bfvec(0, -1);
                true
            }
            'v' => {
                ip.delta = bfvec(0, 1);
                true
            }
            ']' => {
                ip.delta = bfvec(-ip.delta.y, ip.delta.x);
                true
            }
            '[' => {
                ip.delta = bfvec(ip.delta.y, -ip.delta.x);
                true
            }
            '_' => {
                let p = ip.pop();
                ip.delta = if p == T::zero() {
                    bfvec(1, 0)
                } else {
                    bfvec(-1, 0)
                };
                true
            }
            '|' => {
                let p = ip.pop();
                ip.delta = if p == T::zero() {
                    bfvec(0, 1)
                } else {
                    bfvec(0, -1)
                };
                true
            }
            'w' => {
                let b = ip.pop();
                let a = ip.pop();
                if a > b {
                    // ]
                    ip.delta = bfvec(-ip.delta.y, ip.delta.x)
                } else if a < b {
                    // [
                    ip.delta = bfvec(ip.delta.y, -ip.delta.x)
                }
                true
            }
            _ => false,
        }
    }

    fn pop_vector(ip: &mut InstructionPointer<Self, Space>) -> Self {
        let y = ip.pop();
        let x = ip.pop();
        return bfvec(x, y);
    }

    fn push_vector(ip: &mut InstructionPointer<Self, Space>, v: Self) {
        ip.push(v.x);
        ip.push(v.y);
    }
}
