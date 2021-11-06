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

use std::rc::Rc;

use chrono::prelude::Utc;
use hashbrown::HashMap;

use crate::interpreter::instruction_set::{
    sync_instruction, Instruction, InstructionContext, InstructionResult,
};
use crate::interpreter::Funge;

/// The HRTI fingerprint allows a Funge program to measure elapsed time much
/// more finely than the clock values returned by `y`.
///
/// After successfully loading HRTI, the instructions `E`, `G`, `M`, `S`,
/// and `T` take on new semantics.
pub fn load<F: Funge>(ctx: &mut InstructionContext<F>) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('G', sync_instruction(granularity));
    layer.insert('M', sync_instruction(mark));
    layer.insert('T', sync_instruction(timer));
    layer.insert('E', sync_instruction(erase));
    layer.insert('S', sync_instruction(second));
    ctx.ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(ctx: &mut InstructionContext<F>) -> bool {
    ctx.ip.instructions.pop_layer(&['G', 'M', 'T', 'E', 'S'])
}

/// `G` 'Granularity' pushes the smallest clock tick the underlying system
/// can reliably handle, measured in microseconds.
fn granularity<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    ctx.ip.push(1.into());
    InstructionResult::Continue
}

/// `M` 'Mark' designates the timer as having been read by the IP with this
/// ID at this instance in time.
fn mark<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    let ts_micros: i64 = Utc::now().timestamp_nanos() / 1000;
    ctx.ip
        .private_data
        .insert("HRTI.mark".to_owned(), Rc::new(ts_micros));
    InstructionResult::Continue
}

/// `T` 'Timer' pushes the number of microseconds elapsed since the last
/// time an IP with this ID marked the timer. If there is no previous mark,
/// acts like `r`.
fn timer<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(mark) = ctx.ip.private_data.get("HRTI.mark") {
        if let Some(ts_ref) = mark.downcast_ref::<i64>() {
            let ts_micros: i64 = Utc::now().timestamp_nanos() / 1000;
            let ts_diff = ts_micros - *ts_ref;
            ctx.ip.push((ts_diff as i32).into());
        } else {
            ctx.ip.reflect();
        }
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

/// `E` 'Erase mark' erases the last timer mark by this IP (such that `T`
/// above will act like `r`)
fn erase<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    ctx.ip.private_data.remove("HRTI.mark");
    InstructionResult::Continue
}

/// `S` 'Second' pushes the number of microseconds elapsed since the last
/// whole second.
fn second<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    ctx.ip
        .push((Utc::now().timestamp_subsec_micros() as i32).into());
    InstructionResult::Continue
}
