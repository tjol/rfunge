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

use crate::interpreter::{
    instruction_set::{sync_instruction, Instruction},
    Funge, InstructionPointer, InstructionResult,
};

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
pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('D', sync_instruction(sprintf_fpdp));
    layer.insert('F', sync_instruction(sprintf_fpsp));
    layer.insert('I', sync_instruction(sprintf_int));
    layer.insert('L', sync_instruction(sprintf_long));
    layer.insert('S', sync_instruction(sprintf_str));
    ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    ip.instructions.pop_layer(&['D', 'F', 'I', 'L', 'S'][..])
}

fn sprintf_int<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let arg = ip.pop().to_i64().unwrap_or_default();
    let fmt = ip.pop_0gnirts();
    if let Ok(s) = sprintf!(&fmt, arg) {
        ip.push_0gnirts(&s);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn sprintf_long<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
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

fn sprintf_fpdp<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
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

fn sprintf_fpsp<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
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

fn sprintf_str<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let arg = ip.pop_0gnirts();
    let fmt = ip.pop_0gnirts();
    if let Ok(s) = sprintf!(&fmt, arg) {
        ip.push_0gnirts(&s);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}
