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

use crate::fungespace::SrcIO;
use crate::interpreter::instruction_set::{Instruction, InstructionResult, InstructionSet};
use crate::interpreter::MotionCmds;
use crate::{FungeSpace, FungeValue, InstructionPointer, InterpreterEnv};

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
pub fn load<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let mut layer = HashMap::<char, Instruction<Idx, Space, Env>>::new();
    layer.insert('P', put);
    layer.insert('G', get);
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
    instructionset.pop_layer(&['P', 'G'][..])
}

fn put<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let n = ip.pop();
    let va = MotionCmds::pop_vector(ip);
    let vd = MotionCmds::pop_vector(ip);
    
    let mut pos = va;
    let mut remaining = n;
    while remaining > 0.into() {
        space[pos] = ip.pop();
        pos = pos + vd;
        remaining -= 1.into();
    }

    InstructionResult::Continue
}

fn get<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let n = ip.pop();
    let va = MotionCmds::pop_vector(ip);
    let vd = MotionCmds::pop_vector(ip);

    ip.push(0.into());
    
    let mut pos = va;
    let mut remaining = n;
    while remaining > 0.into() {
        ip.push(space[pos]);
        pos = pos + vd;
        remaining -= 1.into();
    }

    InstructionResult::Continue
}


