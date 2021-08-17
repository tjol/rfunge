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

use std::cmp::Ordering;
use std::ops::{Add, Mul, Sub};

use getrandom::getrandom;

use super::ip::InstructionPointer;
use super::InterpreterEnv;
use crate::fungespace::index::{bfvec, BefungeVec};
use crate::fungespace::{FungeIndex, FungeSpace, FungeValue, SrcIO};

pub trait MotionCmds<Space, Env>:
    FungeIndex
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Space::Output, Output = Self>
    + SrcIO<Space>
where
    Space: FungeSpace<Self>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    fn apply_delta(instruction: char, ip: &mut InstructionPointer<Self, Space, Env>) -> bool;
    fn pop_vector_from(stack: &mut Vec<Space::Output>) -> Self;
    fn push_vector_onto(stack: &mut Vec<Space::Output>, v: Self);
    fn pop_vector(ip: &mut InstructionPointer<Self, Space, Env>) -> Self {
        Self::pop_vector_from(ip.stack_mut())
    }
    fn push_vector(ip: &mut InstructionPointer<Self, Space, Env>, v: Self) {
        Self::push_vector_onto(ip.stack_mut(), v)
    }
    fn one_further(&self) -> Self;
}

// Unefunge implementation of MotionCmds
impl<T, Space, Env> MotionCmds<Space, Env> for T
where
    T: FungeValue,
    Space: FungeSpace<Self, Output = T>,
    Env: InterpreterEnv,
{
    fn apply_delta(instruction: char, ip: &mut InstructionPointer<Self, Space, Env>) -> bool {
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
            '?' => {
                let mut rnd = [0_u8; 1];
                getrandom(&mut rnd).ok();
                if rnd[0] & 1 == 1 {
                    ip.delta = T::from(1);
                } else {
                    ip.delta = T::from(-1);
                }
                true
            }
            _ => false,
        }
    }

    fn pop_vector_from(stack: &mut Vec<Space::Output>) -> Self {
        stack.pop().unwrap_or_else(|| 0.into())
    }

    fn push_vector_onto(stack: &mut Vec<Space::Output>, v: Self) {
        stack.push(v);
    }

    fn one_further(&self) -> Self {
        *self + 1.into()
    }
}

// Befunge implementation of MotionCmds
impl<T, Space, Env> MotionCmds<Space, Env> for BefungeVec<T>
where
    Space: FungeSpace<Self, Output = T>,
    T: FungeValue,
    Env: InterpreterEnv,
{
    fn apply_delta(instruction: char, ip: &mut InstructionPointer<Self, Space, Env>) -> bool {
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
                match a.cmp(&b) {
                    Ordering::Greater => ip.delta = bfvec(-ip.delta.y, ip.delta.x),
                    Ordering::Less => ip.delta = bfvec(ip.delta.y, -ip.delta.x),
                    Ordering::Equal => {}
                }
                true
            }
            '?' => {
                let mut rnd = [0_u8; 1];
                getrandom(&mut rnd).ok();
                ip.delta = match rnd[0] & 3 {
                    0 => bfvec(1, 0),
                    1 => bfvec(0, 1),
                    2 => bfvec(-1, 0),
                    _ => bfvec(0, -1),
                };
                true
            }
            _ => false,
        }
    }

    fn pop_vector_from(stack: &mut Vec<Space::Output>) -> Self {
        let y = stack.pop().unwrap_or_else(|| 0.into());
        let x = stack.pop().unwrap_or_else(|| 0.into());
        bfvec(x, y)
    }

    fn push_vector_onto(stack: &mut Vec<Space::Output>, v: Self) {
        stack.push(v.x);
        stack.push(v.y);
    }

    fn one_further(&self) -> Self {
        bfvec(self.x + 1.into(), self.y)
    }
}
