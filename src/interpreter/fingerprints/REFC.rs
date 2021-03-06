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

use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use hashbrown::HashMap;
use num::ToPrimitive;

use crate::interpreter::instruction_set::{sync_instruction, Instruction, InstructionResult};
use crate::interpreter::Funge;
use crate::interpreter::MotionCmds;
use crate::InstructionPointer;

/// From the catseye library
///
/// Fingerprint 0x52454643 ('REFC')
///
/// Under development.
///
/// The REFC fingerprint allows vectors to be encoded into and decoded from
/// single scalar cell values.
///
/// After successfully loading REFC, the instructions `D` and `R` take on
/// new semantics.
///
/// `R` 'Reference' pops a vector off the stack, and pushes a scalar value
/// back onto the stack, unique within an internal list of references, which
/// refers to that vector.
///
/// `D` 'Dereference' pops a scalar value off the stack, and pushes the
/// vector back onto the stack which corresponds to that unique reference
/// value.
///
/// The internal list of references is considered shared among all IP's, so
/// a global static can be used to store this list, so that this extension
/// remains tame.
///
/// This implementation deviates *slightly* from this description: if the
/// fingerprint is loaded twice, independently, by two IPs, the IPs get
/// separate ref lists. (But the ref list is shared between IPs forked off after
/// loading).
pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('R', sync_instruction(reference));
    layer.insert('D', sync_instruction(dereference));
    ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    ip.instructions.pop_layer(&['R', 'D'])
}

fn get_reflist<F: Funge>(ip: &mut InstructionPointer<F>) -> RefMut<Vec<F::Idx>> {
    if !ip.private_data.contains_key("REFC.reflist") {
        ip.private_data.insert(
            "REFC.reflist".to_owned(),
            Rc::new(RefCell::new(Vec::<F::Idx>::new())),
        );
    }
    ip.private_data
        .get("REFC.reflist")
        .and_then(|any_ref| any_ref.downcast_ref::<RefCell<Vec<F::Idx>>>())
        .map(|refcell| refcell.borrow_mut())
        .unwrap()
}

fn reference<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let vec = MotionCmds::pop_vector(ip);
    let ref_idx = {
        let mut rl = get_reflist(ip);
        match rl.iter().position(|v| *v == vec) {
            Some(idx) => (idx as i32).into(),
            None => {
                rl.push(vec);
                (rl.len() as i32 - 1).into()
            }
        }
    };
    ip.push(ref_idx);
    InstructionResult::Continue
}

fn dereference<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    if let Some(vec) = ip
        .pop()
        .to_usize()
        .and_then(|idx| get_reflist(ip).get(idx).copied())
    {
        MotionCmds::push_vector(ip, vec);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}
