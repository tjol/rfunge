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

use super::fungespace::index::{bfvec, BefungeVec64};
use super::fungespace::{FungeIndex, FungeSpace};
use super::ip::InstructionPointer;
use num::ToPrimitive;
use std::fmt::Display;
use std::io;
use std::io::Write;
use std::ops::{Add, Div, Mul, Rem, Sub};

#[derive(Debug, Clone)]
pub enum InstructionResult {
    Continue,
    StayPut,
    Skip,
    Exit,
    Panic,
}

#[derive(Debug, Clone)]
pub enum ProgramResult {
    Ok,
    Panic,
}

pub trait MotionCmds<Space>: FungeIndex + Add<Output = Self> + Mul<i64, Output = Self>
where
    Space: FungeSpace<Self>,
    Space::Output: From<i32> + ToPrimitive + Copy,
{
    fn apply_delta(instruction: char, ip: &mut InstructionPointer<Self, Space>) -> bool;
    fn pop_vector(ip: &mut InstructionPointer<Self, Space>) -> Self;
}

pub struct Interpreter<Idx, Space>
where
    Idx: MotionCmds<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: From<i32>
        + ToPrimitive
        + Add<Output = Space::Output>
        + Sub<Output = Space::Output>
        + Mul<Output = Space::Output>
        + Div<Output = Space::Output>
        + Rem<Output = Space::Output>
        + Copy
        + Display,
{
    pub ips: Vec<InstructionPointer<Idx, Space>>,
    pub space: Space,
}

impl<Idx, Space> Interpreter<Idx, Space>
where
    Idx: MotionCmds<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: From<i32>
        + ToPrimitive
        + Add<Output = Space::Output>
        + Sub<Output = Space::Output>
        + Mul<Output = Space::Output>
        + Div<Output = Space::Output>
        + Rem<Output = Space::Output>
        + Copy
        + Display,
{
    pub fn run(&mut self) -> ProgramResult {
        let last_ip_idx = self.ips.len() - 1;
        let ip = &mut self.ips[last_ip_idx];
        let mut next_instruction = &self.space[ip.location];

        loop {
            let last_result = match next_instruction.to_u32().and_then(char::from_u32) {
                Some('@') => InstructionResult::Exit,
                Some('#') => {
                    // Trampoline
                    ip.location = ip.location + ip.delta;
                    InstructionResult::Skip
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
                Some(':') => {
                    let n = ip.pop();
                    ip.push(n);
                    ip.push(n);
                    InstructionResult::Continue
                }
                Some(digit) if digit >= '0' && digit <= '9' => {
                    ip.push(Space::Output::from((digit as i32) - ('0' as i32)));
                    InstructionResult::Continue
                }
                Some(digit) if digit >= 'a' && digit <= 'f' => {
                    ip.push(Space::Output::from(0xa + (digit as i32) - ('a' as i32)));
                    InstructionResult::Continue
                }
                Some('.') => {
                    print!("{} ", ip.pop());
                    io::stdout().flush().unwrap();
                    InstructionResult::Continue
                }
                Some(',') => {
                    if let Some(c) = ip.pop().to_u32().and_then(char::from_u32) {
                        print!("{}", c);
                    } else {
                        print!("�");
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
                    ip.push(a / b);
                    InstructionResult::Continue
                }
                Some('%') => {
                    let b = ip.pop();
                    let a = ip.pop();
                    ip.push(a % b);
                    InstructionResult::Continue
                }
                Some('j') => {
                    ip.location = ip.location + MotionCmds::pop_vector(ip);
                    InstructionResult::StayPut
                }
                Some('x') => {
                    ip.delta = MotionCmds::pop_vector(ip);
                    InstructionResult::Continue
                }
                Some('r') => {
                    ip.delta = ip.delta * (-1);
                    InstructionResult::Continue
                }
                Some('z') => InstructionResult::Continue,
                Some(c) => {
                    if MotionCmds::apply_delta(c, ip) {
                        InstructionResult::Continue
                    } else {
                        // reflect
                        eprintln!("Unknown instruction: '{}'", c);
                        ip.delta = ip.delta * (-1);
                        InstructionResult::Continue
                    }
                }
                None => {
                    // reflect
                    eprintln!("Unknown non-Unicode instruction!");
                    ip.delta = ip.delta * (-1);
                    InstructionResult::Continue
                }
            };

            match last_result {
                InstructionResult::Continue | InstructionResult::Skip => {
                    // Skip will need special treatment in concurrent funge
                    let (new_loc, new_val) = self.space.move_by(ip.location, ip.delta);
                    ip.location = new_loc;
                    next_instruction = new_val;
                }
                InstructionResult::StayPut => {
                    next_instruction = &self.space[ip.location];
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
impl<Space> MotionCmds<Space> for i64
where
    Space: FungeSpace<Self>,
    Space::Output: From<i32> + ToPrimitive + Copy,
{
    fn apply_delta(instruction: char, ip: &mut InstructionPointer<Self, Space>) -> bool {
        match instruction {
            '>' => {
                ip.delta = 1;
                true
            }
            '<' => {
                ip.delta = -1;
                true
            }
            _ => false,
        }
    }

    fn pop_vector(ip: &mut InstructionPointer<Self, Space>) -> Self {
        ip.pop().to_i64().or(Some(0)).unwrap()
    }
}

// Befunge implementation of MotionCmds
impl<Space> MotionCmds<Space> for BefungeVec64
where
    Space: FungeSpace<Self>,
    Space::Output: From<i32> + ToPrimitive + Copy,
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
            _ => false,
        }
    }

    fn pop_vector(ip: &mut InstructionPointer<Self, Space>) -> Self {
        let y = ip.pop().to_i64().or(Some(0)).unwrap();
        let x = ip.pop().to_i64().or(Some(0)).unwrap();
        return bfvec(x, y);
    }
}
