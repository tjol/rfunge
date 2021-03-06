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

use std::future::Future;
use std::pin::Pin;

use futures_lite::io::AsyncWriteExt;
use hashbrown::HashMap;
use num::ToPrimitive;

use crate::interpreter::{
    instruction_set::{sync_instruction, Instruction},
    Funge, InstructionPointer, InstructionResult,
};
use crate::{FungeValue, InterpreterEnv};

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
pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
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
    layer.insert('P', Instruction::AsyncInstruction(print_fpdp));
    layer.insert('Q', sync_instruction(sqrt));
    layer.insert('R', sync_instruction(conv_str2fpdp));
    layer.insert('S', sync_instruction(sub));
    layer.insert('T', sync_instruction(tan));
    layer.insert('V', sync_instruction(abs));
    layer.insert('X', sync_instruction(exp));
    layer.insert('Y', sync_instruction(pow));
    ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    ip.instructions
        .pop_layer(&"ABCDEFGHIKLMNPQRSTVXY".chars().collect::<Vec<char>>())
}

pub fn ints_to_fpdp(ih: i32, il: i32) -> f64 {
    let i: u64 = (ih as u64 & 0xffffffff) << 32 | (il as u64 & 0xffffffff);
    f64::from_bits(i)
}

pub fn fpdp2ints(f: f64) -> (i32, i32) {
    let i: u64 = f.to_bits();
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

fn conv_int_to_fpdp<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let i = ip.pop();
    let (rh, rl) = fpdp2vals(i.to_f64().unwrap_or_default());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn conv_fpdp2int<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let f = vals_to_fpdp(hi, lo);
    ip.push((f.round() as i32).into());
    InstructionResult::Continue
}

fn conv_str2fpdp<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
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

fn print_fpdp<'a, F: Funge>(
    ip: &'a mut InstructionPointer<F>,
    _space: &'a mut F::Space,
    env: &'a mut F::Env,
) -> Pin<Box<dyn Future<Output = InstructionResult> + 'a>> {
    Box::pin(async move {
        let lo = ip.pop();
        let hi = ip.pop();
        let f = vals_to_fpdp(hi, lo);
        let s = format!("{:.6} ", f);
        if env.output_writer().write(s.as_bytes()).await.is_err() {
            ip.reflect();
        }
        InstructionResult::Continue
    })
}

fn add<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
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

fn sub<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
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

fn mul<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
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

fn div<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
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

fn pow<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
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

fn sin<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let angle = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(angle.sin());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn cos<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let angle = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(angle.cos());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn tan<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let angle = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(angle.tan());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn arcsin<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.asin());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn arccos<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.acos());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn arctan<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.atan());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn ln<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.ln());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn log10<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.log10());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn neg<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(-f);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn sqrt<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.sqrt());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn exp<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.exp());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn abs<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let f = vals_to_fpdp(hi, lo);
    let (rh, rl) = fpdp2vals(f.abs());
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}
