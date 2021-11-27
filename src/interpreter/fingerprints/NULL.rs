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

/// After successfully loading fingerprint 0x4e554c4c, all 26 instructions
/// `A` to `Z` take on the semantics of `r`.
///
/// This can be loaded before loading a regular transparent fingerprint to
/// make it act opaquely.
pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars() {
        layer.insert(c, sync_instruction(reflect));
    }
    ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    ip.instructions
        .pop_layer(&"ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect::<Vec<char>>())
}

fn reflect<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    ip.reflect();
    InstructionResult::Continue
}
