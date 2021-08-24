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

use std::mem::size_of;

use hashbrown::HashMap;

use crate::fungespace::SrcIO;
use crate::interpreter::instruction_set::{Instruction, InstructionResult, InstructionSet};
use crate::interpreter::MotionCmds;
use crate::{FungeSpace, FungeValue, InstructionPointer, InterpreterEnv};

/// From the rcFunge docs:
///
/// "LONG" 0x4c4f4e47
/// A   (ah al bh bl -- rh rl)  Addition
/// B   (ah al -- rh rl)        Absolute value
/// D   (ah al bh bl -- rh rl)  Division
/// E   (n -- rh rl)            Sign extend single to long
/// L   (ah al n -- rh rl)      Shift left n times
/// M   (ah al bh bl -- rh rl)  Multiplication
/// N   (ah al -- rh rl)        Negate
/// O   (ah al bh bl -- rh rl)  Modulo
/// P   (ah al -- )             Print
/// R   (ah al n -- rh rl)      Shift right n times
/// S   (ah al bh bl -- rh rl)  Subraction
/// Z   (0gnirts -- rh rl)      Ascii to long
///
///  * long integers are 2 cell integers, if the interpreter's cell size is 32, then long integers are 64-bits.
///  * Division by zero results in zero, not error
pub fn load<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let mut layer = HashMap::<char, Instruction<Idx, Space, Env>>::new();
    layer.insert('A', add);
    layer.insert('B', abs);
    layer.insert('D', div);
    layer.insert('E', extend);
    layer.insert('L', shift_left);
    layer.insert('M', mul);
    layer.insert('N', neg);
    layer.insert('O', rem);
    layer.insert('P', print_long);
    layer.insert('R', shift_right);
    layer.insert('S', sub);
    layer.insert('Z', parse_long);
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
    instructionset.pop_layer(&"ABDELMNOPRSZ".chars().collect::<Vec<char>>())
}

pub fn val_to_i128<T: FungeValue>(v: T) -> i128 {
    v.to_i128().unwrap_or_default()
}

pub fn vals_to_i128<T: FungeValue>(hi: T, lo: T) -> i128 {
    if size_of::<T>() == 1 {
        val_to_i128(hi) << 32 | val_to_i128(lo)
    } else {
        val_to_i128(hi) << 64 | val_to_i128(lo)
    }
}

pub fn i1282vals<T: FungeValue>(lng: i128) -> (T, T) {
    if size_of::<T>() == 4 {
        let hi = T::from((lng >> 32) as i32);
        let lo = T::from((lng & 0xffffffff) as i32);
        (hi, lo)
    } else {
        let hi = T::from_i64((lng >> 64) as i64).unwrap_or_else(|| 0.into());
        let lo = T::from_i64((lng & 0xffffffffffffffff) as i64).unwrap_or_else(|| 0.into());
        (hi, lo)
    }
}

fn extend<Idx, Space, Env>(
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
    let lng = val_to_i128(ip.pop());
    let (hi, lo) = i1282vals(lng);
    ip.push(hi);
    ip.push(lo);
    InstructionResult::Continue
}

fn print_long<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let lo = ip.pop();
    let hi = ip.pop();
    let lng = vals_to_i128(hi, lo);
    if write!(env.output_writer(), "{} ", lng).is_err() {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn parse_long<Idx, Space, Env>(
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
    let s = ip.pop_0gnirts();
    let lng: i128 = s.parse().unwrap_or_default();
    let (hi, lo) = i1282vals(lng);
    ip.push(hi);
    ip.push(lo);
    InstructionResult::Continue
}

fn abs<Idx, Space, Env>(
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
    let lng = vals_to_i128(hi, lo);
    let (hi, lo) = i1282vals(lng.abs());
    ip.push(hi);
    ip.push(lo);
    InstructionResult::Continue
}

fn neg<Idx, Space, Env>(
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
    let lng = vals_to_i128(hi, lo);
    let (hi, lo) = i1282vals(-lng);
    ip.push(hi);
    ip.push(lo);
    InstructionResult::Continue
}

fn add<Idx, Space, Env>(
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
    let bl = ip.pop();
    let bh = ip.pop();
    let al = ip.pop();
    let ah = ip.pop();
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a + b);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn sub<Idx, Space, Env>(
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
    let bl = ip.pop();
    let bh = ip.pop();
    let al = ip.pop();
    let ah = ip.pop();
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a - b);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn mul<Idx, Space, Env>(
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
    let bl = ip.pop();
    let bh = ip.pop();
    let al = ip.pop();
    let ah = ip.pop();
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a * b);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn div<Idx, Space, Env>(
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
    let bl = ip.pop();
    let bh = ip.pop();
    let al = ip.pop();
    let ah = ip.pop();
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a / b);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn rem<Idx, Space, Env>(
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
    let bl = ip.pop();
    let bh = ip.pop();
    let al = ip.pop();
    let ah = ip.pop();
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a % b);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn shift_left<Idx, Space, Env>(
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
    let n = val_to_i128(ip.pop());
    let al = ip.pop();
    let ah = ip.pop();
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a << n);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn shift_right<Idx, Space, Env>(
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
    let n = val_to_i128(ip.pop());
    let al = ip.pop();
    let ah = ip.pop();
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a >> n);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}
