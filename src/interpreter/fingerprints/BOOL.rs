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

use crate::interpreter::{
    instruction_set::{sync_instruction, Instruction},
    Funge, InstructionPointer, InstructionResult,
};

pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('A', sync_instruction(and));
    layer.insert('O', sync_instruction(or));
    layer.insert('N', sync_instruction(not));
    layer.insert('X', sync_instruction(xor));
    ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    ip.instructions.pop_layer(&['A', 'O', 'N', 'X'])
}

pub(super) fn and<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = ip.pop();
    let a = ip.pop();
    ip.push(a & b);
    InstructionResult::Continue
}

pub(super) fn or<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = ip.pop();
    let a = ip.pop();
    ip.push(a | b);
    InstructionResult::Continue
}

fn not<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let n = ip.pop();
    ip.push(!n);
    InstructionResult::Continue
}

pub(super) fn xor<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let b = ip.pop();
    let a = ip.pop();
    ip.push(a ^ b);
    InstructionResult::Continue
}
