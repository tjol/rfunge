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
mod FPDP;
mod FPRT;
mod FPSP;
mod FRTH;
mod HRTI;
mod JSTR;
mod LONG;
mod MODU;
mod NULL;
mod REFC;
mod ROMA;

#[cfg(not(target_arch = "wasm32"))]
mod SOCK;

use super::{Funge, InstructionSet};

/// Convert a fingerprint string to a numeric fingerprint
pub fn string_to_fingerprint(fpr_str: &str) -> i32 {
    let mut fpr = 0;
    for c in fpr_str.chars() {
        fpr *= 256;
        fpr += c as u32
    }
    fpr as i32
}

/// Get a list of all available fingerprints that are considered "safe" (i.e.,
/// no executing external commands, no IO)
pub fn safe_fingerprints() -> Vec<i32> {
    vec![
        string_to_fingerprint("NULL"),
        string_to_fingerprint("BOOL"),
        string_to_fingerprint("HRTI"),
        string_to_fingerprint("FIXP"),
        string_to_fingerprint("ROMA"),
        string_to_fingerprint("MODU"),
        string_to_fingerprint("REFC"),
        string_to_fingerprint("FPSP"),
        string_to_fingerprint("FPDP"),
        string_to_fingerprint("LONG"),
        string_to_fingerprint("FPRT"),
        string_to_fingerprint("JSTR"),
        string_to_fingerprint("FRTH"),
    ]
}

/// Get a list of all available fingerprints
pub fn all_fingerprints() -> Vec<i32> {
    let mut fprts = safe_fingerprints();
    if cfg!(not(target_arch = "wasm32")) {
        fprts.push(string_to_fingerprint("SOCK"));
    }
    fprts
}

pub fn load<F: Funge>(instructionset: &mut InstructionSet<F>, fpr: i32) -> bool {
    if fpr == string_to_fingerprint("NULL") {
        NULL::load(instructionset)
    } else if fpr == string_to_fingerprint("BOOL") {
        BOOL::load(instructionset)
    } else if fpr == string_to_fingerprint("HRTI") {
        HRTI::load(instructionset)
    } else if fpr == string_to_fingerprint("FIXP") {
        FIXP::load(instructionset)
    } else if fpr == string_to_fingerprint("ROMA") {
        ROMA::load(instructionset)
    } else if fpr == string_to_fingerprint("MODU") {
        MODU::load(instructionset)
    } else if fpr == string_to_fingerprint("REFC") {
        REFC::load(instructionset)
    } else if fpr == string_to_fingerprint("FPSP") {
        FPSP::load(instructionset)
    } else if fpr == string_to_fingerprint("FPDP") {
        FPDP::load(instructionset)
    } else if fpr == string_to_fingerprint("LONG") {
        LONG::load(instructionset)
    } else if fpr == string_to_fingerprint("FPRT") {
        FPRT::load(instructionset)
    } else if fpr == string_to_fingerprint("JSTR") {
        JSTR::load(instructionset)
    } else if fpr == string_to_fingerprint("FRTH") {
        FRTH::load(instructionset)
    } else {
        load_platform_specific(instructionset, fpr)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_platform_specific<F: Funge>(instructionset: &mut InstructionSet<F>, fpr: i32) -> bool {
    if fpr == string_to_fingerprint("SOCK") {
        SOCK::load(instructionset)
    } else {
        false
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_platform_specific<F: Funge>(
    _instructionset: &mut InstructionSet<F>,
    _fpr: i32,
) -> bool {
    false
}

pub fn unload<F: Funge>(instructionset: &mut InstructionSet<F>, fpr: i32) -> bool {
    if fpr == string_to_fingerprint("NULL") {
        NULL::unload(instructionset)
    } else if fpr == string_to_fingerprint("BOOL") {
        BOOL::unload(instructionset)
    } else if fpr == string_to_fingerprint("HRTI") {
        HRTI::unload(instructionset)
    } else if fpr == string_to_fingerprint("FIXP") {
        FIXP::unload(instructionset)
    } else if fpr == string_to_fingerprint("ROMA") {
        ROMA::unload(instructionset)
    } else if fpr == string_to_fingerprint("MODU") {
        MODU::unload(instructionset)
    } else if fpr == string_to_fingerprint("REFC") {
        REFC::unload(instructionset)
    } else if fpr == string_to_fingerprint("FPSP") {
        FPSP::unload(instructionset)
    } else if fpr == string_to_fingerprint("FPDP") {
        FPDP::unload(instructionset)
    } else if fpr == string_to_fingerprint("LONG") {
        LONG::unload(instructionset)
    } else if fpr == string_to_fingerprint("FPRT") {
        FPRT::unload(instructionset)
    } else if fpr == string_to_fingerprint("JSTR") {
        JSTR::unload(instructionset)
    } else if fpr == string_to_fingerprint("FRTH") {
        FRTH::unload(instructionset)
    } else {
        unload_platform_specific(instructionset, fpr)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn unload_platform_specific<F: Funge>(
    instructionset: &mut InstructionSet<F>,
    fpr: i32,
) -> bool {
    if fpr == string_to_fingerprint("SOCK") {
        SOCK::unload(instructionset)
    } else {
        false
    }
}

#[cfg(target_arch = "wasm32")]
pub fn unload_platform_specific<F: Funge>(
    _instructionset: &mut InstructionSet<F>,
    _fpr: i32,
) -> bool {
    false
}
