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

use hashbrown::HashMap;
use num::ToPrimitive;
use sprintf::sprintf;

use super::FPDP::vals_to_fpdp;
use super::FPSP::val_to_fpsp;
use super::LONG::vals_to_i128;

use crate::fungespace::SrcIO;
use crate::interpreter::instruction_set::{Instruction, InstructionResult, InstructionSet};
use crate::interpreter::MotionCmds;
use crate::{FungeSpace, FungeValue, InstructionPointer, InterpreterEnv};

/// From the rcFunge docs:
///
/// "FPRT" 0x46505254
/// D   (fmt fh fl -- 0gnirts)      Format FPDP type number
/// F   (fmt f -- 0gnirts)          Format FPSP type number
/// I   (fmt i -- 0gnirts)          Format an integer
/// L   (fmt h l -- 0gnirts)        Format a long integer
/// S   (fmt 0gnirts -- 0gnirts)    Format a string
///
/// Formats are printf style
/// Error in any function reflects
pub fn load<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let mut layer = HashMap::<char, Instruction<Idx, Space, Env>>::new();
    layer.insert('D', sprintf_fpdp);
    layer.insert('F', sprintf_fpsp);
    layer.insert('I', sprintf_int);
    layer.insert('L', sprintf_long);
    layer.insert('S', sprintf_str);
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
    instructionset.pop_layer(&['D', 'F', 'I', 'L', 'S'][..])
}

fn sprintf_int<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let arg = ip.pop().to_i64().unwrap_or_default();
    let fmt = ip.pop_0gnirts();
    if let Ok(s) = sprintf!(&fmt, arg) {
        ip.push_0gnirts(&s);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn sprintf_long<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let lo = ip.pop();
    let hi = ip.pop();
    let arg = vals_to_i128(hi, lo) as i64; // sprintf does not support i128
    let fmt = ip.pop_0gnirts();
    if let Ok(s) = sprintf!(&fmt, arg) {
        ip.push_0gnirts(&s);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn sprintf_fpdp<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let lo = ip.pop();
    let hi = ip.pop();
    let arg = vals_to_fpdp(hi, lo);
    let fmt = ip.pop_0gnirts();
    if let Ok(s) = sprintf!(&fmt, arg) {
        ip.push_0gnirts(&s);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn sprintf_fpsp<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let i = ip.pop();
    let arg = val_to_fpsp(i); // sprintf does not support i128
    let fmt = ip.pop_0gnirts();
    if let Ok(s) = sprintf!(&fmt, arg) {
        ip.push_0gnirts(&s);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn sprintf_str<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let arg = ip.pop_0gnirts();
    let fmt = ip.pop_0gnirts();
    if let Ok(s) = sprintf!(&fmt, arg) {
        ip.push_0gnirts(&s);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}
