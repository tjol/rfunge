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

//! This module contains only complex instructions; most instructions are
//! built into the interpreter

use std::cmp::{max, min};

use num::ToPrimitive;

use super::instruction_set::exec_instruction;
use super::ip::InstructionPointer;
use super::motion::MotionCmds;
use super::{InstructionResult, InterpreterEnv};
use crate::fungespace::{FungeSpace, FungeValue};

pub fn iterate<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let n = ip.pop();
    let (mut new_loc, new_val_ref) = space.move_by(ip.location, ip.delta);
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
                let old_loc = ip.location;
                ip.location = new_loc;
                exec_instruction(new_val, ip, space, env);
                let (new_loc2, new_val_ref) = space.move_by(ip.location, ip.delta);
                new_loc = new_loc2;
                new_val = *new_val_ref;
                ip.location = old_loc;
                new_val_c = new_val.to_char();
            }
            for _ in 0..n {
                match exec_instruction(new_val, ip, space, env) {
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

pub fn begin_block<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    if let Some(n) = ip.pop().to_isize() {
        // take n items off the SOSS (old TOSS)
        let n_to_take = max(0, min(n, ip.stack().len() as isize));
        let zeros_for_toss = max(0, n - n_to_take);
        let zeros_for_soss = max(0, -n);

        let split_idx = ip.stack().len() - n_to_take as usize;
        let mut transfer_elems = ip.stack_mut().split_off(split_idx);

        for _ in 0..zeros_for_soss {
            ip.push(0.into());
        }

        MotionCmds::push_vector(ip, ip.storage_offset); // onto SOSS / old TOSS

        // create a new stack
        ip.stack_stack.push(Vec::new());

        for _ in 0..zeros_for_toss {
            ip.push(0.into());
        }

        ip.stack_mut().append(&mut transfer_elems);

        ip.storage_offset = ip.location + ip.delta;
    } else {
        // reflect
        ip.delta = ip.delta * (-1).into();
    }

    InstructionResult::Continue
}

pub fn end_block<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    if ip.stack_stack.len() > 1 {
        if let Some(n) = ip.pop().to_isize() {
            let mut toss = ip.stack_stack.pop().unwrap();

            // restore the storage offset
            ip.storage_offset = MotionCmds::pop_vector(ip);

            let n_to_take = max(0, min(n, toss.len() as isize));
            let zeros_for_soss = max(0, n - n_to_take);
            let n_to_pop = max(0, -n);

            if n_to_pop > 0 {
                for _ in 0..n_to_pop {
                    ip.pop();
                }
            } else {
                for _ in 0..zeros_for_soss {
                    ip.push(0.into());
                }

                let split_idx = toss.len() - n_to_take as usize;
                ip.stack_mut().append(&mut toss.split_off(split_idx));
            }
        } else {
            // reflect
            ip.delta = ip.delta * (-1).into();
        }
    } else {
        // reflect
        ip.delta = ip.delta * (-1).into();
    }

    InstructionResult::Continue
}

pub fn stack_under_stack<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let nstacks = ip.stack_stack.len();
    if nstacks > 1 {
        if let Some(n) = ip.pop().to_isize() {
            if n > 0 {
                for _ in 0..n {
                    let v = ip.stack_stack[nstacks - 2].pop().unwrap_or(0.into());
                    ip.push(v);
                }
            } else if n < 0 {
                for _ in 0..(-n) {
                    let v = ip.pop();
                    ip.stack_stack[nstacks - 2].push(v);
                }
            }
        } else {
            // reflect
            ip.delta = ip.delta * (-1).into();
        }
    } else {
        // reflect
        ip.delta = ip.delta * (-1).into();
    }
    InstructionResult::Continue
}
