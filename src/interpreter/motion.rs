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

use std::ops::{Add, Mul};

use super::ip::InstructionPointer;
use super::InterpreterEnv;
use crate::fungespace::index::{bfvec, BefungeVec};
use crate::fungespace::{FungeIndex, FungeSpace, FungeValue};

pub trait MotionCmds<Space, Env>:
    FungeIndex + Add<Output = Self> + Mul<Space::Output, Output = Self>
where
    Space: FungeSpace<Self>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    fn apply_delta(instruction: char, ip: &mut InstructionPointer<Self, Space, Env>) -> bool;
    fn pop_vector(ip: &mut InstructionPointer<Self, Space, Env>) -> Self;
    fn push_vector(ip: &mut InstructionPointer<Self, Space, Env>, v: Self);
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
            _ => false,
        }
    }

    fn pop_vector(ip: &mut InstructionPointer<Self, Space, Env>) -> Self {
        ip.pop()
    }

    fn push_vector(ip: &mut InstructionPointer<Self, Space, Env>, v: Self) {
        ip.push(v);
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

    fn pop_vector(ip: &mut InstructionPointer<Self, Space, Env>) -> Self {
        let y = ip.pop();
        let x = ip.pop();
        return bfvec(x, y);
    }

    fn push_vector(ip: &mut InstructionPointer<Self, Space, Env>, v: Self) {
        ip.push(v.x);
        ip.push(v.y);
    }
}
