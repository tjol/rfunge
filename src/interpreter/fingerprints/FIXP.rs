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

use std::collections::HashMap;
use std::f64::consts::{FRAC_1_PI, PI};

use num::{ToPrimitive, Signed};

use super::string_to_fingerprint;
use super::BOOL;
use crate::fungespace::SrcIO;
use crate::interpreter::instruction_set::{Instruction, InstructionResult, InstructionSet};
use crate::interpreter::MotionCmds;
use crate::{FungeSpace, FungeValue, InstructionPointer, InterpreterEnv};

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
pub fn load<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let mut layer = HashMap::<char, Instruction<Idx, Space, Env>>::new();
    layer.insert('A', BOOL::and);
    layer.insert('B', arccos);
    layer.insert('C', cos);
    layer.insert('D', rnd);
    layer.insert('I', sin);
    layer.insert('J', arcsin);
    layer.insert('N', neg);
    layer.insert('O', BOOL::or);
    layer.insert('P', mulpi);
    layer.insert('Q', sqrt);
    layer.insert('R', pow);
    layer.insert('S', sgn);
    layer.insert('T', tan);
    layer.insert('U', arctan);
    layer.insert('V', abs);
    layer.insert('X', BOOL::xor);
    instructionset.add_layer(string_to_fingerprint("FIXP"), layer);
    true
}

pub fn unload<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    // Check that this fingerprint is on top
    if instructionset.top_fingerprint() == string_to_fingerprint("FIXP") {
        instructionset.pop_layer();
        true
    } else {
        false
    }
}

fn rad2deg(angle: f64) -> f64 {
    angle * FRAC_1_PI * 180.
}

fn deg2rad(angle: f64) -> f64 {
    angle * PI / 180.
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
    let radians = (ip.pop().to_f64().unwrap_or(0.) / 10000.).acos();
    ip.push(((rad2deg(radians) * 10000.).round() as i32).into());
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
    let radians = deg2rad(ip.pop().to_f64().unwrap_or(0.) / 10000.);
    ip.push(((radians.cos() * 10000.).round() as i32).into());
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
    let radians = (ip.pop().to_f64().unwrap_or(0.) / 10000.).asin();
    ip.push(((rad2deg(radians) * 10000.).round() as i32).into());
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
    let radians = deg2rad(ip.pop().to_f64().unwrap_or(0.) / 10000.);
    ip.push(((radians.sin() * 10000.).round() as i32).into());
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
    let radians = (ip.pop().to_f64().unwrap_or(0.) / 10000.).atan();
    ip.push(((rad2deg(radians) * 10000.).round() as i32).into());
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
    let radians = deg2rad(ip.pop().to_f64().unwrap_or(0.) / 10000.);
    ip.push(((radians.tan() * 10000.).round() as i32).into());
    InstructionResult::Continue
}

fn rnd<Idx, Space, Env>(
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
    let limit = ip.pop();
    let sgn = limit.signum();
    let abs_limit = (limit * sgn).to_i32().unwrap_or_else(i32::max_value);
    let number = if abs_limit == 0 {
        0.into()
    } else {
        let rndnum = rand::random::<f64>() * (abs_limit as f64);
        Space::Output::from(rndnum as i32) * sgn
    };

    ip.push(number);
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
    let n = ip.pop();
    ip.push(-n);
    InstructionResult::Continue
}

fn mulpi<Idx, Space, Env>(
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
    let n = ip.pop().to_f64().unwrap_or_default() * PI;
    ip.push((n as i32).into());
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
    let n = ip.pop().to_f64().unwrap_or_default().sqrt();
    ip.push((n as i32).into());
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
    let b = ip.pop().to_i32().unwrap_or_default();
    let a = ip.pop().to_f64().unwrap_or_default();
    ip.push((a.powi(b).round() as i32).into());
    InstructionResult::Continue
}

fn sgn<Idx, Space, Env>(
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
    let n = ip.pop();
    ip.push(n.signum());
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
    let n = ip.pop();
    ip.push(n * n.signum());
    InstructionResult::Continue
}
