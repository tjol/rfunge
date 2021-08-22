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

#![allow(non_snake_case)]

mod BOOL;
mod FIXP;
mod HRTI;

use super::{InstructionSet, InterpreterEnv, MotionCmds};
use crate::fungespace::{FungeSpace, FungeValue, SrcIO};

pub fn string_to_fingerprint(fpr_str: &str) -> i32 {
    let mut fpr = 0;
    for c in fpr_str.chars() {
        fpr *= 256;
        fpr += c as u32
    }
    fpr as i32
}

pub fn safe_fingerprints() -> Vec<i32> {
    vec![
        string_to_fingerprint("BOOL"),
        string_to_fingerprint("HRTI"),
        string_to_fingerprint("FIXP"),
    ]
}

pub fn all_fingerprints() -> Vec<i32> {
    safe_fingerprints()
}

pub fn load<Idx, Space, Env>(instructionset: &mut InstructionSet<Idx, Space, Env>, fpr: i32) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    if fpr == string_to_fingerprint("BOOL") {
        BOOL::load(instructionset)
    } else if fpr == string_to_fingerprint("HRTI") {
        HRTI::load(instructionset)
    } else if fpr == string_to_fingerprint("FIXP") {
        FIXP::load(instructionset)
    } else {
        false
    }
}

pub fn unload<Idx, Space, Env>(
    instructionset: &mut InstructionSet<Idx, Space, Env>,
    fpr: i32,
) -> bool
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    if fpr == string_to_fingerprint("BOOL") {
        BOOL::unload(instructionset)
    } else if fpr == string_to_fingerprint("HRTI") {
        HRTI::unload(instructionset)
    } else if fpr == string_to_fingerprint("FIXP") {
        FIXP::unload(instructionset)
    } else {
        false
    }
}
