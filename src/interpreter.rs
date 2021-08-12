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

use std::io::{Read, Write};
use std::ops::{Add, Mul};

use num::ToPrimitive;

use super::fungespace::index::{bfvec, BefungeVec};
use super::fungespace::{FungeIndex, FungeSpace, FungeValue};
use super::ip::{InstructionMode, InstructionPointer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionResult {
    Continue,
    StayPut,
    Skip,
    Exit,
    Panic,
}

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
}

pub struct Interpreter<'a, Idx, Space>
where
    Idx: MotionCmds<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
{
    pub ips: Vec<InstructionPointer<Idx, Space>>,
    pub space: Space,
    pub env: InterpreterEnvironment<'a>,
}

pub struct InterpreterEnvironment<'a> {
    pub input: &'a mut dyn Read,
    pub output: &'a mut dyn Write,
    pub warn: &'a mut dyn FnMut(&str),
    pub io_mode: IOMode,
}

impl<'a, Idx, Space> Interpreter<'a, Idx, Space>
where
    Idx: MotionCmds<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
{
    pub fn run(&mut self) -> ProgramResult {
        let ip_idx = self.ips.len() - 1;
        let mut next_instruction = self.space[self.ips[ip_idx].location];

        loop {
            let ip = &self.ips[ip_idx];
            let instr_mode = ip.instructions.mode;
            let result = match instr_mode {
                InstructionMode::Normal => self.exec_instr(ip_idx, next_instruction),
                InstructionMode::String => self.read_string(ip_idx, next_instruction),
            };

            let ip = &mut self.ips[ip_idx];

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

    fn exec_instr(&mut self, ip_idx: usize, raw_instruction: Space::Output) -> InstructionResult {
        let ip = &mut self.ips[ip_idx];
        match raw_instruction.try_to_char() {
            Some('@') => InstructionResult::Exit,
            Some('#') => {
                // Trampoline
                ip.location = ip.location + ip.delta;
                InstructionResult::Continue
            }
            Some(';') => {
                loop {
                    let (new_loc, new_val) = self.space.move_by(ip.location, ip.delta);
                    ip.location = new_loc;
                    if Some(';') == new_val.to_u32().and_then(char::from_u32) {
                        break;
                    }
                }
                InstructionResult::Skip
            }
            Some('$') => {
                ip.pop();
                InstructionResult::Continue
            }
            Some('n') => {
                ip.stack_mut().drain(0..);
                InstructionResult::Continue
            }
            Some('\\') => {
                let a = ip.pop();
                let b = ip.pop();
                ip.push(a);
                ip.push(b);
                InstructionResult::Continue
            }
            Some(':') => {
                let n = ip.pop();
                ip.push(n);
                ip.push(n);
                InstructionResult::Continue
            }
            Some(digit) if digit >= '0' && digit <= '9' => {
                ip.push(((digit as i32) - ('0' as i32)).into());
                InstructionResult::Continue
            }
            Some(digit) if digit >= 'a' && digit <= 'f' => {
                ip.push((0xa + (digit as i32) - ('a' as i32)).into());
                InstructionResult::Continue
            }
            Some('"') => {
                ip.instructions.mode = InstructionMode::String;
                ip.location = ip.location + ip.delta;
                InstructionResult::StayPut
            }
            Some('\'') => {
                let loc = ip.location + ip.delta;
                ip.push(self.space[loc]);
                ip.location = loc;
                InstructionResult::Continue
            }
            Some('s') => {
                let loc = ip.location + ip.delta;
                self.space[loc] = ip.pop();
                ip.location = loc;
                InstructionResult::Continue
            }
            Some('.') => {
                if write!(self.env.output, "{} ", ip.pop()).is_err() {
                    self.warn("IO Error");
                }
                InstructionResult::Continue
            }
            Some(',') => {
                let c = ip.pop();
                if match self.env.io_mode {
                    IOMode::Text => write!(self.env.output, "{}", c.to_char()),
                    IOMode::Binary => self
                        .env
                        .output
                        .write(&[(c & 0xff.into()).to_u8().unwrap()])
                        .and_then(|_| Ok(())),
                }
                .is_err()
                {
                    self.warn("IO Error");
                }
                InstructionResult::Continue
            }
            Some('+') => {
                let b = ip.pop();
                let a = ip.pop();
                ip.push(a + b);
                InstructionResult::Continue
            }
            Some('-') => {
                let b = ip.pop();
                let a = ip.pop();
                ip.push(a - b);
                InstructionResult::Continue
            }
            Some('*') => {
                let b = ip.pop();
                let a = ip.pop();
                ip.push(a * b);
                InstructionResult::Continue
            }
            Some('/') => {
                let b = ip.pop();
                let a = ip.pop();
                ip.push(if b != 0.into() { a / b } else { 0.into() });
                InstructionResult::Continue
            }
            Some('%') => {
                let b = ip.pop();
                let a = ip.pop();
                ip.push(if b != 0.into() { a % b } else { 0.into() });
                InstructionResult::Continue
            }
            Some('`') => {
                let b = ip.pop();
                let a = ip.pop();
                ip.push(if a > b { 1.into() } else { 0.into() });
                InstructionResult::Continue
            }
            Some('!') => {
                let v = ip.pop();
                ip.push(if v == 0.into() { 1.into() } else { 0.into() });
                InstructionResult::Continue
            }
            Some('j') => {
                ip.location = ip.location + ip.delta * ip.pop();
                InstructionResult::Continue
            }
            Some('x') => {
                ip.delta = MotionCmds::pop_vector(ip);
                InstructionResult::Continue
            }
            Some('p') => {
                let loc = MotionCmds::pop_vector(ip);
                self.space[loc] = ip.pop();
                InstructionResult::Continue
            }
            Some('g') => {
                let loc = MotionCmds::pop_vector(ip);
                ip.push(self.space[loc]);
                InstructionResult::Continue
            }
            Some('k') => {
                let n = ip.pop();
                let (mut new_loc, new_val_ref) = self.space.move_by(ip.location, ip.delta);
                let mut new_val = *new_val_ref;
                let mut loop_result = InstructionResult::Continue;
                if let Some(n) = n.to_isize() {
                    if n <= 0 {
                        // surprising behaviour! 1k leads to the next instruction
                        // being executed twice, 0k to it being skipped
                        ip.location = new_loc;
                        loop_result = InstructionResult::Continue;
                    } else {
                        let mut new_val_c = new_val.to_char();
                        while new_val_c == ';' {
                            // skip what must be skipped
                            // fake-execute!
                            let ip = &mut self.ips[ip_idx];
                            let old_loc = ip.location;
                            ip.location = new_loc;
                            self.exec_instr(ip_idx, new_val);
                            let ip = &mut self.ips[ip_idx];
                            let (new_loc2, new_val_ref) = self.space.move_by(ip.location, ip.delta);
                            new_loc = new_loc2;
                            new_val = *new_val_ref;
                            ip.location = old_loc;
                            new_val_c = new_val.to_char();
                        }
                        for _ in 0..n {
                            match self.exec_instr(ip_idx, new_val) {
                                InstructionResult::Continue => {}
                                res => {
                                    loop_result = res;
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    // Reflect on overflow
                    ip.delta = ip.delta * (-1).into();
                }
                loop_result
            }
            Some('r') => {
                ip.delta = ip.delta * (-1).into();
                InstructionResult::Continue
            }
            Some('z') => InstructionResult::Continue,
            Some(c) => {
                if MotionCmds::apply_delta(c, ip) {
                    InstructionResult::Continue
                } else {
                    // reflect
                    ip.delta = ip.delta * (-1).into();
                    self.warn(&format!("Unknown instruction: '{}'", c));
                    InstructionResult::Continue
                }
            }
            None => {
                // reflect
                ip.delta = ip.delta * (-1).into();
                self.warn("Unknown non-Unicode instruction!");
                InstructionResult::Continue
            }
        }
    }

    fn read_string(&mut self, ip_idx: usize, raw_instruction: Space::Output) -> InstructionResult {
        let ip = &mut self.ips[ip_idx];
        match raw_instruction.to_u32().and_then(char::from_u32) {
            Some('"') => {
                ip.instructions.mode = InstructionMode::Normal;
                InstructionResult::Continue
            }
            Some(' ') => {
                ip.push(raw_instruction);
                // skip over the following spaces
                InstructionResult::Continue
            }
            Some(_) | None => {
                // Some other character
                ip.push(raw_instruction);
                // Do not skip over the following spaces
                ip.location = ip.location + ip.delta;
                InstructionResult::StayPut
            }
        }
    }

    fn warn(&mut self, msg: &str) {
        (self.env.warn)(msg)
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
}
