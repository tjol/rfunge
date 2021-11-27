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

use divrem::DivRem;
use hashbrown::HashMap;

use crate::interpreter::{
    instruction_set::{sync_instruction, Instruction},
    Funge, InstructionPointer, InstructionResult,
};

/// From the catseye library
///
/// Fingerprint 0x4d4f4455 ('MODU')
///
/// Under development.
///
/// The MODU fingerprint implements some of the finer, less-well-agreed-upon
/// points of modulo arithmetic. With positive arguments, these instructions
/// work exactly the same as % does. However, when negative values are involved,
/// they all work differently:
///
/// M: signed-result modulo:
///
/// U: Sam Holden's unsigned-result modulo
///
/// R: C-language integer remainder
///
/// Interpretation:
///
/// For all definitions of the remainder, the following must hold:
/// given
///     n / d = q rem r
/// then
///     q * d + r = n
///
/// C uses truncating division: *q* is rounded towards zero, and *r* is chosen
/// accordingly. This is indusputably what the `R` instruction does.
///
/// `R` *is* a signed-result modulo operator, but as `M` is supposed to do
/// something different, we shall use a floor division remainder for `M`.
/// (*q* is rounded toward -∞). This is what CCBI, cfunge and pyfunge do;
/// pyfunge even uses this as its default (as does Python). rcFunge does
/// something mathematically unsound here.
///
/// `U` is interpreted as the Euclidian remainder: round *q* such that *r* > 0.
/// This is what CCBI does; cfunge, pyfunge, and, again, rcfunge, do something
/// mathematically unsound (they return the absolute value of the C remainder).
pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('M', sync_instruction(signed_rem));
    layer.insert('U', sync_instruction(unsigned_rem));
    layer.insert('R', sync_instruction(c_rem));
    ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    ip.instructions.pop_layer(&['M', 'U', 'R'])
}

fn signed_rem<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = ip.pop();
    let a = ip.pop();
    if b == 0.into() {
        ip.push(0.into());
    } else {
        let (q, r) = a.div_rem(b); // truncating
        ip.push(if q < 0.into() { r + b } else { r });
    }
    InstructionResult::Continue
}

fn unsigned_rem<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = ip.pop();
    let a = ip.pop();
    if b == 0.into() {
        ip.push(0.into());
    } else {
        let r = a % b; // truncating
        ip.push(if r < 0.into() {
            if b > 0.into() {
                r + b
            } else {
                -b + r
            }
        } else {
            r
        });
    }
    InstructionResult::Continue
}

fn c_rem<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = ip.pop();
    let a = ip.pop();
    if b == 0.into() {
        ip.push(0.into());
    } else {
        ip.push(a % b); // default in Rust
    }
    InstructionResult::Continue
}
