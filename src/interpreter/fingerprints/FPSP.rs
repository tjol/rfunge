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
/// "FPSP" 0x46505350
/// A    (a b -- n)     Add two single precision fp numbers
/// B    (n -- n)       Sin of single precision fp number
/// C    (n -- n)       Cosin of single precision fp number
/// D    (a b -- n)     Divide two single precision fp numbers
/// E    (n -- n)       Arcsin of single precision fp number
/// F    (n -- n)       Convert integer to floating point
/// G    (n -- n)       Arctangent of single precision fp number
/// H    (n -- n)       Arccosin of single precision fp number
/// I    (n -- n)       Convert floating point to integer
/// K    (n -- n)       Natural logarithm of single precision fp number
/// L    (n -- n)       Base 10 logarithm of single precision fp number
/// M    (a b -- n)     Multiply two single precision fp numbers
/// N    (n -- n)       Negate single precision fp number
/// P    (n -- )        Print a floating point number
/// Q    (n -- n)       Single precision square root
/// R    (0gnirts -- n) Convert ascii number to floating point
/// S    (a b -- n)     Subtract two single precision fp numbers
/// T    (n -- n)       Tangent of single precision fp number
/// V    (n -- n)       Absolute value of single precision fp number
/// X    (n -- n)       Exponential of single precision fp number (e**n)
/// Y    (x y -- n)     Raise x to the power of y
///
/// Trig functions work in radians
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
    layer.insert('F', sync_instruction(conv_int_to_fpsp));
    layer.insert('G', sync_instruction(arctan));
    layer.insert('H', sync_instruction(arccos));
    layer.insert('I', sync_instruction(conv_fpsp2int));
    layer.insert('K', sync_instruction(ln));
    layer.insert('L', sync_instruction(log10));
    layer.insert('M', sync_instruction(mul));
    layer.insert('N', sync_instruction(neg));
    layer.insert('P', Instruction::AsyncInstruction(print_fpsp));
    layer.insert('Q', sync_instruction(sqrt));
    layer.insert('R', sync_instruction(conv_str2fpsp));
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

pub fn int_to_fpsp(i: i32) -> f32 {
    unsafe { *((&i as *const i32) as *const f32) }
}

pub fn fpsp2int(f: f32) -> i32 {
    unsafe { *((&f as *const f32) as *const i32) }
}

pub fn val_to_fpsp<T: FungeValue>(i: T) -> f32 {
    int_to_fpsp(i.to_i32().unwrap_or_default())
}

pub fn fpsp2val<T: FungeValue>(f: f32) -> T {
    fpsp2int(f).into()
}

fn conv_int_to_fpsp<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let i = ip.pop();
    ip.push(fpsp2val(i.to_f32().unwrap_or_default()));
    InstructionResult::Continue
}

fn conv_fpsp2int<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let f = val_to_fpsp(ip.pop());
    ip.push((f.round() as i32).into());
    InstructionResult::Continue
}

fn conv_str2fpsp<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let s = ip.pop_0gnirts();
    if let Ok(f) = s.parse() {
        ip.push(fpsp2val(f));
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn print_fpsp<'a, F: Funge>(
    ip: &'a mut InstructionPointer<F>,
    _space: &'a mut F::Space,
    env: &'a mut F::Env,
) -> Pin<Box<dyn Future<Output = InstructionResult> + 'a>> {
    Box::pin(async move {
        let f = val_to_fpsp(ip.pop());
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
    let b = val_to_fpsp(ip.pop());
    let a = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(a + b));
    InstructionResult::Continue
}

fn sub<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = val_to_fpsp(ip.pop());
    let a = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(a - b));
    InstructionResult::Continue
}

fn mul<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = val_to_fpsp(ip.pop());
    let a = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(a * b));
    InstructionResult::Continue
}

fn div<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = val_to_fpsp(ip.pop());
    let a = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(a / b));
    InstructionResult::Continue
}

fn pow<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = val_to_fpsp(ip.pop());
    let a = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(a.powf(b)));
    InstructionResult::Continue
}

fn sin<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let angle = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(angle.sin()));
    InstructionResult::Continue
}

fn cos<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let angle = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(angle.cos()));
    InstructionResult::Continue
}

fn tan<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let angle = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(angle.tan()));
    InstructionResult::Continue
}

fn arcsin<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let f = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(f.asin()));
    InstructionResult::Continue
}

fn arccos<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let f = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(f.acos()));
    InstructionResult::Continue
}

fn arctan<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let f = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(f.atan()));
    InstructionResult::Continue
}

fn ln<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let f = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(f.ln()));
    InstructionResult::Continue
}

fn log10<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let f = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(f.log10()));
    InstructionResult::Continue
}

fn neg<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let f = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(-f));
    InstructionResult::Continue
}

fn sqrt<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let f = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(f.sqrt()));
    InstructionResult::Continue
}

fn exp<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let f = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(f.exp()));
    InstructionResult::Continue
}

fn abs<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let f = val_to_fpsp(ip.pop());
    ip.push(fpsp2val(f.abs()));
    InstructionResult::Continue
}
