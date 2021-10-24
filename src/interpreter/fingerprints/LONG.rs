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

use futures_lite::io::AsyncWriteExt;
use hashbrown::HashMap;

use crate::interpreter::instruction_set::{
    async_instruction, sync_instruction, Instruction, InstructionContext, InstructionResult,
    InstructionSet,
};
use crate::interpreter::Funge;
use crate::{FungeValue, InterpreterEnv};

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
pub fn load<F: Funge>(instructionset: &mut InstructionSet<F>) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('A', sync_instruction(add));
    layer.insert('B', sync_instruction(abs));
    layer.insert('D', sync_instruction(div));
    layer.insert('E', sync_instruction(extend));
    layer.insert('L', sync_instruction(shift_left));
    layer.insert('M', sync_instruction(mul));
    layer.insert('N', sync_instruction(neg));
    layer.insert('O', sync_instruction(rem));
    layer.insert('P', async_instruction(print_long));
    layer.insert('R', sync_instruction(shift_right));
    layer.insert('S', sync_instruction(sub));
    layer.insert('Z', sync_instruction(parse_long));
    instructionset.add_layer(layer);
    true
}

pub fn unload<F: Funge>(instructionset: &mut InstructionSet<F>) -> bool {
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

fn extend<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let lng = val_to_i128(ctx.ip.pop());
    let (hi, lo) = i1282vals(lng);
    ctx.ip.push(hi);
    ctx.ip.push(lo);
    InstructionResult::Continue
}

async fn print_long<F: Funge>(mut ctx: InstructionContext<F>) -> (InstructionContext<F>, InstructionResult)
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let lng = vals_to_i128(hi, lo);
    let s = format!("{} ", lng);
    if ctx.env.output_writer().write(s.as_bytes()).await.is_err() {
        ctx.ip.reflect();
    }
    (ctx, InstructionResult::Continue)
}

fn parse_long<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let s = ctx.ip.pop_0gnirts();
    let lng: i128 = s.parse().unwrap_or_default();
    let (hi, lo) = i1282vals(lng);
    ctx.ip.push(hi);
    ctx.ip.push(lo);
    InstructionResult::Continue
}

fn abs<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let lng = vals_to_i128(hi, lo);
    let (hi, lo) = i1282vals(lng.abs());
    ctx.ip.push(hi);
    ctx.ip.push(lo);
    InstructionResult::Continue
}

fn neg<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let lo = ctx.ip.pop();
    let hi = ctx.ip.pop();
    let lng = vals_to_i128(hi, lo);
    let (hi, lo) = i1282vals(-lng);
    ctx.ip.push(hi);
    ctx.ip.push(lo);
    InstructionResult::Continue
}

fn add<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let bl = ctx.ip.pop();
    let bh = ctx.ip.pop();
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a + b);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    InstructionResult::Continue
}

fn sub<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let bl = ctx.ip.pop();
    let bh = ctx.ip.pop();
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a - b);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    InstructionResult::Continue
}

fn mul<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let bl = ctx.ip.pop();
    let bh = ctx.ip.pop();
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a * b);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    InstructionResult::Continue
}

fn div<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let bl = ctx.ip.pop();
    let bh = ctx.ip.pop();
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a / b);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    InstructionResult::Continue
}

fn rem<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let bl = ctx.ip.pop();
    let bh = ctx.ip.pop();
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a % b);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    InstructionResult::Continue
}

fn shift_left<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let n = val_to_i128(ctx.ip.pop());
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a << n);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    InstructionResult::Continue
}

fn shift_right<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let n = val_to_i128(ctx.ip.pop());
    let al = ctx.ip.pop();
    let ah = ctx.ip.pop();
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a >> n);
    ctx.ip.push(rh);
    ctx.ip.push(rl);
    InstructionResult::Continue
}
