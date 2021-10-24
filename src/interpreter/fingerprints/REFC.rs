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

use crate::fungespace::SrcIO;
use crate::interpreter::instruction_set::{
    sync_instruction, Instruction, InstructionContext, InstructionResult, InstructionSet,
};
use crate::interpreter::MotionCmds;
use crate::{FungeSpace, FungeValue, InstructionPointer, InterpreterEnv};

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
pub fn load<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let mut layer = HashMap::<char, Instruction<Idx, Space, Env>>::new();
    layer.insert('R', sync_instruction(reference));
    layer.insert('D', sync_instruction(dereference));
    instructionset.add_layer(layer);
    true
}

pub fn unload<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    instructionset.pop_layer(&['R', 'D'])
}

fn get_reflist<Idx, Space, Env>(ip: &mut InstructionPointer<Idx, Space, Env>) -> RefMut<Vec<Idx>>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    if !ip.private_data.contains_key("REFC.reflist") {
        ip.private_data.insert(
            "REFC.reflist".to_owned(),
            Rc::new(RefCell::new(Vec::<Idx>::new())),
        );
    }
    ip.private_data
        .get("REFC.reflist")
        .and_then(|any_ref| any_ref.downcast_ref::<RefCell<Vec<Idx>>>())
        .map(|refcell| refcell.borrow_mut())
        .unwrap()
}

fn reference<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    let vec = MotionCmds::pop_vector(&mut ctx.ip);
    let ref_idx = {
        let mut rl = get_reflist(&mut ctx.ip);
        match rl.iter().position(|v| *v == vec) {
            Some(idx) => (idx as i32).into(),
            None => {
                rl.push(vec);
                (rl.len() as i32 - 1).into()
            }
        }
    };
    ctx.ip.push(ref_idx);
    (ctx, InstructionResult::Continue)
}

fn dereference<Idx, Space, Env>(
    mut ctx: InstructionContext<Idx, Space, Env>,
) -> (InstructionContext<Idx, Space, Env>, InstructionResult)
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space> + 'static,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue + 'static,
    Env: InterpreterEnv + 'static,
{
    if let Some(vec) = ctx
        .ip
        .pop()
        .to_usize()
        .and_then(|idx| get_reflist(&mut ctx.ip).get(idx).copied())
    {
        MotionCmds::push_vector(&mut ctx.ip, vec);
    } else {
        ctx.ip.reflect();
    }
    (ctx, InstructionResult::Continue)
}
