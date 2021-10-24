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

use futures_lite::io::AsyncWriteExt;
use hashbrown::HashMap;
use num::ToPrimitive;

use crate::fungespace::SrcIO;
use crate::interpreter::instruction_set::{
    async_instruction, sync_instruction, Instruction, InstructionContext, InstructionResult,
    InstructionSet,
};
use crate::interpreter::MotionCmds;
use crate::{FungeSpace, FungeValue, InterpreterEnv};

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
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let mut layer = HashMap::<char, Instruction<Idx, Space, Env>>::new();
    layer.insert('A', sync_instruction(add));
    layer.insert('B', sync_instruction(sin));
    layer.insert('C', sync_instruction(cos));
    layer.insert('D', sync_instruction(div));
    layer.insert('E', sync_instruction(arcsin));
    layer.insert('F', sync_instruction(conv_int_to_fpdp));
    layer.insert('G', sync_instruction(arctan));
    layer.insert('H', sync_instruction(arccos));
    layer.insert('I', sync_instruction(conv_fpdp2int));
    layer.insert('K', sync_instruction(ln));
    layer.insert('L', sync_instruction(log10));
    layer.insert('M', sync_instruction(mul));
    layer.insert('N', sync_instruction(neg));
    layer.insert('P', async_instruction(print_fpdp));
    layer.insert('Q', sync_instruction(sqrt));
    layer.insert('R', sync_instruction(conv_str2fpdp));
    layer.insert('S', sync_instruction(sub));
    layer.insert('T', sync_instruction(tan));
    layer.insert('V', sync_instruction(abs));
    layer.insert('X', sync_instruction(exp));
    layer.insert('Y', sync_instruction(pow));
    instructionset.add_layer(layer);
    true
}

pub fn unload<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
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
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let i = ctx.ip.pop();
    let (rh, rl) = fpdp2vals(i.to_f64().unwrap_or_default());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn conv_fpdp2int<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    ctx.ip.push((f.round() as i32).into());
    (ctx, InstructionResult::Continue)
}

fn conv_str2fpdp<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let s = ctx.ip.pop_0gnirts();
    if let Ok(f) = s.parse() {
        let (rh, rl) = fpdp2vals(f);
        ctx.ip.push(rh);
        ctx.ip.push(rl);
    } else {
        ctx.ip.reflect();
    }
    (ctx, InstructionResult::Continue)
}

async fn print_fpdp<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let s = format!("{:.6} ", f);
    if ctx.env.output_writer().write(s.as_bytes()).await.is_err() {
        ctx.ip.reflect();
    }
    (ctx, InstructionResult::Continue)
}

fn add<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let bl = ctx.ip.pop();
    let bh = ctx.ip.pop();
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let b = vals_to_fpdp(bh, bl);
    let a = vals_to_fpdp(ah, al);
    let (rh, rl) = fpdp2vals(a + b);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn sub<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let bl = ctx.ip.pop();
    let bh = ctx.ip.pop();
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let b = vals_to_fpdp(bh, bl);
    let a = vals_to_fpdp(ah, al);
    let (rh, rl) = fpdp2vals(a - b);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn mul<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let bl = ctx.ip.pop();
    let bh = ctx.ip.pop();
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let b = vals_to_fpdp(bh, bl);
    let a = vals_to_fpdp(ah, al);
    let (rh, rl) = fpdp2vals(a * b);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn div<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let bl = ctx.ip.pop();
    let bh = ctx.ip.pop();
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let b = vals_to_fpdp(bh, bl);
    let a = vals_to_fpdp(ah, al);
    let (rh, rl) = fpdp2vals(a / b);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn pow<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let bl = ctx.ip.pop();
    let bh = ctx.ip.pop();
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let b = vals_to_fpdp(bh, bl);
    let a = vals_to_fpdp(ah, al);
    let (rh, rl) = fpdp2vals(a.powf(b));
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn sin<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let angle = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(angle.sin());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn cos<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let angle = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(angle.cos());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn tan<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let angle = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(angle.tan());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn arcsin<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.asin());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn arccos<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.acos());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn arctan<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.atan());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn ln<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.ln());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn log10<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.log10());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn neg<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(-f);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn sqrt<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.sqrt());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn exp<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.exp());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}

fn abs<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.abs());
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    (ctx, InstructionResult::Continue)
}
