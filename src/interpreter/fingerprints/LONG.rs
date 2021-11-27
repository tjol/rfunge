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
use std::mem::size_of;
use std::pin::Pin;

use futures_lite::io::AsyncWriteExt;
use hashbrown::HashMap;

use crate::interpreter::{
    instruction_set::{sync_instruction, Instruction},
    Funge, InstructionPointer, InstructionResult,
};
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
pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('A', sync_instruction(add));
    layer.insert('B', sync_instruction(abs));
    layer.insert('D', sync_instruction(div));
    layer.insert('E', sync_instruction(extend));
    layer.insert('L', sync_instruction(shift_left));
    layer.insert('M', sync_instruction(mul));
    layer.insert('N', sync_instruction(neg));
    layer.insert('O', sync_instruction(rem));
    layer.insert('P', Instruction::AsyncInstruction(print_long));
    layer.insert('R', sync_instruction(shift_right));
    layer.insert('S', sync_instruction(sub));
    layer.insert('Z', sync_instruction(parse_long));
    ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    ip.instructions
        .pop_layer(&"ABDELMNOPRSZ".chars().collect::<Vec<char>>())
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

fn extend<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lng = val_to_i128(ip.pop());
    let (hi, lo) = i1282vals(lng);
    ip.push(hi);
    ip.push(lo);
    InstructionResult::Continue
}

fn print_long<'a, F: Funge>(
    ip: &'a mut InstructionPointer<F>,
    _space: &'a mut F::Space,
    env: &'a mut F::Env,
) -> Pin<Box<dyn Future<Output = InstructionResult> + 'a>> {
    Box::pin(async move {
        let lo = ip.pop();
        let hi = ip.pop();
        let lng = vals_to_i128(hi, lo);
        let s = format!("{} ", lng);
        if env.output_writer().write(s.as_bytes()).await.is_err() {
            ip.reflect();
        }
        InstructionResult::Continue
    })
}

fn parse_long<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let s = ip.pop_0gnirts();
    let lng: i128 = s.parse().unwrap_or_default();
    let (hi, lo) = i1282vals(lng);
    ip.push(hi);
    ip.push(lo);
    InstructionResult::Continue
}

fn abs<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let lng = vals_to_i128(hi, lo);
    let (hi, lo) = i1282vals(lng.abs());
    ip.push(hi);
    ip.push(lo);
    InstructionResult::Continue
}

fn neg<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let lo = ip.pop();
    let hi = ip.pop();
    let lng = vals_to_i128(hi, lo);
    let (hi, lo) = i1282vals(-lng);
    ip.push(hi);
    ip.push(lo);
    InstructionResult::Continue
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
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a + b);
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
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a - b);
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
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a * b);
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
    let b = vals_to_i128(bh, bl);
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a / b);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn rem<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
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

fn shift_left<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let n = val_to_i128(ip.pop());
    let al = ip.pop();
    let ah = ip.pop();
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a << n);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}

fn shift_right<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let n = val_to_i128(ip.pop());
    let al = ip.pop();
    let ah = ip.pop();
    let a = vals_to_i128(ah, al);
    let (rh, rl) = i1282vals(a >> n);
    ip.push(rh);
    ip.push(rl);
    InstructionResult::Continue
}
