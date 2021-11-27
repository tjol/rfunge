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

use std::f64::consts::{FRAC_1_PI, PI};

use hashbrown::HashMap;
use num::{Signed, ToPrimitive};

use super::BOOL;
use crate::interpreter::{
    instruction_set::{sync_instruction, Instruction},
    Funge, InstructionPointer, InstructionResult,
};

/// From the rcFunge docs:
///
/// "FIXP" 0x4649585
/// A    (a b -- a and b)    And
/// B    (n -- arccos(b))    Find arccosin of tos
/// C    (n -- cos(b))       Find cosin of tos
/// D    (n -- rnd(n))       RanDom number
/// I    (n -- sin(b))       Find sin of tos
/// J    (n -- arcsin(b))    Find arcsin of tos
/// N    (a -- 0-a)          Negate
/// O    (a b -- a or b)     Or
/// P    (a -- a*pi)         Multiply by pi
/// Q    (a -- sqrt(a))      Square root
/// R    (a b -- a**b)       Raise a to the power of b
/// S    (n -- n)            Replace tos with sign of tos
/// T    (n -- tan(b))       Find tangent of tos
/// U    (n -- arctan(b)     Find arctangent of tos
/// V    (n -- n)            Absolute value of tos
/// X    (a b -- a xor b)    Xor
///
/// The functions C,I,T,B,J,U expect their arguments times 10000, for example:
/// 45 should be passed as 450000. The results will also be multiplied by 10000,
/// thereby giving 4 digits of decimal precision.
///
/// Trigonometric functions work in degrees. not radians.
pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('A', sync_instruction(BOOL::and));
    layer.insert('B', sync_instruction(arccos));
    layer.insert('C', sync_instruction(cos));
    layer.insert('D', sync_instruction(rnd));
    layer.insert('I', sync_instruction(sin));
    layer.insert('J', sync_instruction(arcsin));
    layer.insert('N', sync_instruction(neg));
    layer.insert('O', sync_instruction(BOOL::or));
    layer.insert('P', sync_instruction(mulpi));
    layer.insert('Q', sync_instruction(sqrt));
    layer.insert('R', sync_instruction(pow));
    layer.insert('S', sync_instruction(sgn));
    layer.insert('T', sync_instruction(tan));
    layer.insert('U', sync_instruction(arctan));
    layer.insert('V', sync_instruction(abs));
    layer.insert('X', sync_instruction(BOOL::xor));
    ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    ip.instructions
        .pop_layer(&"ABCDIJNOPQRSTUVX".chars().collect::<Vec<char>>())
}

fn rad2deg(angle: f64) -> f64 {
    angle * FRAC_1_PI * 180.
}

fn deg2rad(angle: f64) -> f64 {
    angle * PI / 180.
}

fn arccos<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let radians = (ip.pop().to_f64().unwrap_or(0.) / 10000.).acos();
    ip.push(((rad2deg(radians) * 10000.).round() as i32).into());
    InstructionResult::Continue
}

fn cos<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let radians = deg2rad(ip.pop().to_f64().unwrap_or(0.) / 10000.);
    ip.push(((radians.cos() * 10000.).round() as i32).into());
    InstructionResult::Continue
}

fn arcsin<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let radians = (ip.pop().to_f64().unwrap_or(0.) / 10000.).asin();
    ip.push(((rad2deg(radians) * 10000.).round() as i32).into());
    InstructionResult::Continue
}

fn sin<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let radians = deg2rad(ip.pop().to_f64().unwrap_or(0.) / 10000.);
    ip.push(((radians.sin() * 10000.).round() as i32).into());
    InstructionResult::Continue
}

fn arctan<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let radians = (ip.pop().to_f64().unwrap_or(0.) / 10000.).atan();
    ip.push(((rad2deg(radians) * 10000.).round() as i32).into());
    InstructionResult::Continue
}

fn tan<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let radians = deg2rad(ip.pop().to_f64().unwrap_or(0.) / 10000.);
    ip.push(((radians.tan() * 10000.).round() as i32).into());
    InstructionResult::Continue
}

fn rnd<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let limit = ip.pop();
    let sgn = limit.signum();
    let abs_limit = (limit * sgn).to_i32().unwrap_or_else(i32::max_value);
    let number = if abs_limit == 0 {
        0.into()
    } else {
        let rndnum = rand::random::<f64>() * (abs_limit as f64);
        F::Value::from(rndnum as i32) * sgn
    };

    ip.push(number);
    InstructionResult::Continue
}

fn neg<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let n = ip.pop();
    ip.push(-n);
    InstructionResult::Continue
}

fn mulpi<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let n = ip.pop().to_f64().unwrap_or_default() * PI;
    ip.push((n as i32).into());
    InstructionResult::Continue
}

fn sqrt<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let n = ip.pop().to_f64().unwrap_or_default().sqrt();
    ip.push((n as i32).into());
    InstructionResult::Continue
}

fn pow<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = ip.pop().to_i32().unwrap_or_default();
    let a = ip.pop().to_f64().unwrap_or_default();
    ip.push((a.powi(b).round() as i32).into());
    InstructionResult::Continue
}

fn sgn<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let n = ip.pop();
    ip.push(n.signum());
    InstructionResult::Continue
}

fn abs<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let n = ip.pop();
    ip.push(n * n.signum());
    InstructionResult::Continue
}
