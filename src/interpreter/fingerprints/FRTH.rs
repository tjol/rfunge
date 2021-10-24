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

use hashbrown::HashMap;
use num::{FromPrimitive, ToPrimitive, Zero};

use crate::fungespace::SrcIO;
use crate::interpreter::instruction_set::{
    sync_instruction, Instruction, InstructionContext, InstructionResult, InstructionSet,
};
use crate::interpreter::MotionCmds;
use crate::{FungeSpace, FungeValue, InterpreterEnv};

/// From the rcFunge docs
///
/// D   ( .. -- .. n)       Push depth of stack to tos
/// L   ( .. n -- .. n)     Forth Roll command
/// O   (a b -- a b a)      Forth Over command
/// P   (.. n -- .. n)      Forth Pick command
/// R   (a b c -- b c a)    Forth Rot command
///
/// Stack operations are subject to the modes set by MODE
///
/// Clarification
///
///  * P should reflect on a negative argument
///  * P should push 0 if argument is greater than stack size
///  * L should act like forth -roll with a negative argument
///  * L with an argument larger than the stack size is allowed, enough
///    zeroes will be created in order to fulfill the request. Example:
///    n543210a-L will leave a stack of: 2 3 4 5 0 0 0 0 0 0 1
///  * L,P the top of stack is position 0
pub fn load<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let mut layer = HashMap::<char, Instruction<Idx, Space, Env>>::new();
    layer.insert('D', sync_instruction(depth));
    layer.insert('L', sync_instruction(roll));
    layer.insert('O', sync_instruction(over));
    layer.insert('P', sync_instruction(pick));
    layer.insert('R', sync_instruction(rot));
    instructionset.add_layer(layer);
    true
}

pub fn unload<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    instructionset.pop_layer(&['D', 'L', 'O', 'P', 'R'][..])
}

fn depth<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    ctx.ip
        .push(FromPrimitive::from_usize(ctx.ip.stack().len()).unwrap_or_else(Zero::zero));

    (ctx, InstructionResult::Continue)
}

fn roll<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let stack = ctx.ip.stack_mut();
    let u = stack.pop().and_then(|v| v.to_isize()).unwrap_or_default();
    match u.cmp(&Zero::zero()) {
        Ordering::Greater => {
            // roll mode
            let u = u as usize;
            let l = stack.len();
            let v = if u < l {
                stack.remove(l - 1 - u)
            } else {
                Zero::zero()
            };
            ctx.ip.push(v);
        }
        Ordering::Less => {
            // -roll mode
            let u = (-u) as usize;
            let v = stack.pop().unwrap_or_else(Zero::zero);
            while stack.len() < u {
                stack.insert(0, Zero::zero());
            }
            stack.insert(stack.len() - u, v);
        }
        _ => {}
    }
    (ctx, InstructionResult::Continue)
}

fn over<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let stack = ctx.ip.stack();
    let v = if stack.len() >= 2 {
        stack[stack.len() - 2]
    } else {
        Zero::zero()
    };
    ctx.ip.push(v);

    (ctx, InstructionResult::Continue)
}

fn pick<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let u = ctx.ip.pop();
    if u < Zero::zero() {
        ctx.ip.reflect()
    } else {
        let u = u.to_usize().unwrap_or_default();
        let stack = ctx.ip.stack();
        let l = stack.len();
        let v = if u < l {
            stack[l - 1 - u]
        } else {
            Zero::zero()
        };
        ctx.ip.push(v);
    }

    (ctx, InstructionResult::Continue)
}

fn rot<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let stack = ctx.ip.stack_mut();
    let l = stack.len();
    let v = if l >= 3 {
        stack.remove(l - 3)
    } else {
        Zero::zero()
    };
    ctx.ip.push(v);

    (ctx, InstructionResult::Continue)
}
