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

use crate::interpreter::instruction_set::{
    sync_instruction, Instruction, InstructionContext, InstructionResult, InstructionSet,
};
use crate::interpreter::MotionCmds;
use crate::interpreter::Funge;

/// From https://web.archive.org/web/20070525220700/http://www.jess2.net:80/code/funge/myexts.txt
///
/// "JSTR" 0x4a535452
/// P (Vd Va n -- )         pop n cells off of the stack and write at Va with delta
///                         Vd.
/// G (Vd Va n -- 0gnirts)  read n cells from position Va and delta Vd, push on
///                         stack as a string.
///
/// NOTE: The rcFunge docs swap `G` and `P`, but rcFunge still implements the
/// fingerprint as documented here!
pub fn load<F: Funge>(instructionset: &mut InstructionSet<F>) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('P', sync_instruction(put));
    layer.insert('G', sync_instruction(get));
    instructionset.add_layer(layer);
    true
}

pub fn unload<F: Funge>(instructionset: &mut InstructionSet<F>) -> bool {
    instructionset.pop_layer(&['P', 'G'][..])
}

fn put<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let n = ctx.ip.pop();
    let va = MotionCmds::pop_vector(&mut ctx.ip);
    let vd = MotionCmds::pop_vector(&mut ctx.ip);

    let mut pos = va + ctx.ip.storage_offset;
    let mut remaining = n;
    while remaining > 0.into() {
        ctx.space[pos] = ctx.ip.pop();
        pos = pos + vd;
        remaining -= 1.into();
    }

    InstructionResult::Continue
}

fn get<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let n = ctx.ip.pop();
    let va = MotionCmds::pop_vector(&mut ctx.ip);
    let vd = MotionCmds::pop_vector(&mut ctx.ip);

    ctx.ip.push(0.into());

    let mut pos = va + ctx.ip.storage_offset;
    let mut remaining = n;
    while remaining > 0.into() {
        ctx.ip.push(ctx.space[pos]);
        pos = pos + vd;
        remaining -= 1.into();
    }

    InstructionResult::Continue
}
