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

use crate::fungespace::SrcIO;
use crate::interpreter::instruction_set::{Instruction, InstructionResult, InstructionSet};
use crate::interpreter::MotionCmds;
use crate::{FungeSpace, FungeValue, InstructionPointer, InterpreterEnv};

/// From the rcFunge docs:
///
/// "FPDP" 0x46504450
/// A    (a b -- n)     Add two double precision fp numbers
/// B    (n -- n)       Sin of double precision fp number
/// C    (n -- n)       Cosin of double precision fp number
/// D    (a b -- n)     Divide two double precision fp numbers
/// E    (n -- n)       Arcsin of double precision fp number
/// F    (n -- n)       Convert integer to floating point
/// G    (n -- n)       Arctangent of double precision fp number
/// H    (n -- n)       Arccosin of double precision fp number
/// I    (n -- n)       Convert floating point to integer
/// K    (n -- n)       Natural logarithm of double precision fp number
/// L    (n -- n)       Base 10 logarithm of double precision fp number
/// M    (a b -- n)     Multiply two double precision fp numbers
/// N    (n -- n)       Negate double precision fp number
/// P    (n -- )        Print a floating point number
/// Q    (n -- n)       Double precision square root
/// R    (0gnirts -- n) Convert ascii number to floating point
/// S    (a b -- n)     Subtract two double precision fp numbers
/// T    (n -- n)       Tangent of double precision fp number
/// V    (n -- n)       Absolute value of double precision fp number
/// X    (n -- n)       Exponential of double precision fp number (e**n)
/// Y    (x y -- n)     Raise x to the power of y
///
/// The docs do not mention whether these instructions operator on one or two
/// stack cells per double. We're using two cells even in 64 bit mode for
/// compatibility (following the behaviour of the other implementations).
pub fn load<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let mut layer = HashMap::<char, Instruction<Idx, Space, Env>>::new();
    layer.insert('A', add);
    layer.insert('B', sin);
    layer.insert('C', cos);
    layer.insert('D', div);
    layer.insert('E', arcsin);
    layer.insert('F', conv_int_to_fpdp);
    layer.insert('G', arctan);
    layer.insert('H', arccos);
    layer.insert('I', conv_fpdp2int);
    layer.insert('K', ln);
    layer.insert('L', log10);
    layer.insert('M', mul);
    layer.insert('N', neg);
    layer.insert('P', print_fpdp);
    layer.insert('Q', sqrt);
    layer.insert('R', conv_str2fpdp);
    layer.insert('S', sub);
    layer.insert('T', tan);
    layer.insert('V', abs);
    layer.insert('X', exp);
    layer.insert('Y', pow);
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
    instructionset.pop_layer(&"ABCDEFGHIKLMNPQRSTVXY".chars().collect::<Vec<char>>())
}

pub fn ints_to_fpdp(ih: i32, il: i32) -> f64 {
    let i: u64 = (ih as u64 & 0xffffffff) << 32 | (il as u64 & 0xffffffff);
    unsafe { *((&i as *const u64) as *const f64) }
}

pub fn fpdp2ints(f: f64) -> (i32, i32) {
    let i: u64 = unsafe { *((&f as *const f64) as *const u64) };
    ((i >> 32) as i32, (i & 0xffffffff) as i32)
}

pub fn vals_to_fpdp<T: FungeValue>(hi: T, lo: T) -> f64 {
    ints_to_fpdp(
        hi.to_i32().unwrap_or_default(),
        lo.to_i32().unwrap_or_default(),
    )
}

pub fn fpdp2vals<T: FungeValue>(f: f64) -> (T, T) {
    let (ih, il) = fpdp2ints(f);
    (ih.into(), il.into())
}

fn conv_int_to_fpdp<Idx, Space, Env>(
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
    let (rh, rl) = fpdp2vals(i.to_f64().unwrap_or_default());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn conv_fpdp2int<Idx, Space, Env>(
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
    let f = vals_to_fpdp(hi, lo);
    ip.push((f.round() as i32).into());
    InstructionResult::Continue
}

fn conv_str2fpdp<Idx, Space, Env>(
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
    if let Ok(f) = s.parse() {
        let (rh, rl) = fpdp2vals(f);
        ip.push(rh);
        ip.push(rl);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn print_fpdp<Idx, Space, Env>(
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
    let f = vals_to_fpdp(hi, lo);
    if write!(env.output_writer(), "{:.6} ", f).is_err() {
        ip.reflect();
    }
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
    let b = vals_to_fpdp(bh, bl);
    let a = vals_to_fpdp(ah, al);
    let (rh, rl) = fpdp2vals(a + b);
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
    let b = vals_to_fpdp(bh, bl);
    let a = vals_to_fpdp(ah, al);
    let (rh, rl) = fpdp2vals(a - b);
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
    let b = vals_to_fpdp(bh, bl);
    let a = vals_to_fpdp(ah, al);
    let (rh, rl) = fpdp2vals(a * b);
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
    let b = vals_to_fpdp(bh, bl);
    let a = vals_to_fpdp(ah, al);
    let (rh, rl) = fpdp2vals(a / b);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn pow<Idx, Space, Env>(
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
    let b = vals_to_fpdp(bh, bl);
    let a = vals_to_fpdp(ah, al);
    let (rh, rl) = fpdp2vals(a.powf(b));
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn sin<Idx, Space, Env>(
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
    let angle = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(angle.sin());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn cos<Idx, Space, Env>(
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
    let angle = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(angle.cos());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn tan<Idx, Space, Env>(
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
    let angle = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(angle.tan());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn arcsin<Idx, Space, Env>(
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
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.asin());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn arccos<Idx, Space, Env>(
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
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.acos());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn arctan<Idx, Space, Env>(
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
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.atan());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn ln<Idx, Space, Env>(
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
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.ln());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn log10<Idx, Space, Env>(
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
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.log10());
    ip.push(rh);
    ip.push(rl);
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
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(-f);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn sqrt<Idx, Space, Env>(
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
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.sqrt());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn exp<Idx, Space, Env>(
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
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.exp());
    ip.push(rh);
    ip.push(rl);
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
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.abs());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}
