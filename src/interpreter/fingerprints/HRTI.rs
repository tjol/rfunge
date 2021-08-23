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
use std::rc::Rc;

use chrono::prelude::Utc;

use crate::fungespace::SrcIO;
use crate::interpreter::instruction_set::{Instruction, InstructionResult, InstructionSet};
use crate::interpreter::MotionCmds;
use crate::{FungeSpace, FungeValue, InstructionPointer, InterpreterEnv};

/// The HRTI fingerprint allows a Funge program to measure elapsed time much
/// more finely than the clock values returned by `y`.
///
/// After successfully loading HRTI, the instructions `E`, `G`, `M`, `S`,
/// and `T` take on new semantics.
pub fn load<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let mut layer = HashMap::<char, Instruction<Idx, Space, Env>>::new();
    layer.insert('G', granularity);
    layer.insert('M', mark);
    layer.insert('T', timer);
    layer.insert('E', erase);
    layer.insert('S', second);
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
    instructionset.pop_layer(&['G', 'M', 'T', 'E', 'S'])
}

/// `G` 'Granularity' pushes the smallest clock tick the underlying system
/// can reliably handle, measured in microseconds.
fn granularity<Idx, Space, Env>(
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
    ip.push(1.into());
    InstructionResult::Continue
}

/// `M` 'Mark' designates the timer as having been read by the IP with this
/// ID at this instance in time.
fn mark<Idx, Space, Env>(
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
    let ts_micros: i64 = Utc::now().timestamp_nanos() / 1000;
    ip.private_data
        .insert("HRTI.mark".to_owned(), Rc::new(ts_micros));
    InstructionResult::Continue
}

/// `T` 'Timer' pushes the number of microseconds elapsed since the last
/// time an IP with this ID marked the timer. If there is no previous mark,
/// acts like `r`.
fn timer<Idx, Space, Env>(
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
    if let Some(mark) = ip.private_data.get("HRTI.mark") {
        if let Some(ts_ref) = mark.downcast_ref::<i64>() {
            let ts_micros: i64 = Utc::now().timestamp_nanos() / 1000;
            let ts_diff = ts_micros - *ts_ref;
            ip.push((ts_diff as i32).into());
        } else {
            ip.reflect();
        }
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

/// `E` 'Erase mark' erases the last timer mark by this IP (such that `T`
/// above will act like `r`)
fn erase<Idx, Space, Env>(
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
    ip.private_data.remove("HRTI.mark");
    InstructionResult::Continue
}

/// `S` 'Second' pushes the number of microseconds elapsed since the last
/// whole second.
fn second<Idx, Space, Env>(
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
    ip.push((Utc::now().timestamp_subsec_micros() as i32).into());
    InstructionResult::Continue
}
