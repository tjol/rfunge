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
pub mod TURT;

#[cfg(not(target_family = "wasm"))]
mod SOCK;

#[cfg(not(target_family = "wasm"))]
mod TERM;

use super::{Funge, InstructionContext};

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
    let mut fprts = vec![
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
    ];
    if cfg!(not(target_family = "wasm")) {
        fprts.push(string_to_fingerprint("TERM"));
    }
    fprts
}

/// Get a list of all available fingerprints
pub fn all_fingerprints() -> Vec<i32> {
    let mut fprts = safe_fingerprints();
    fprts.push(string_to_fingerprint("TURT"));
    if cfg!(not(target_family = "wasm")) {
        fprts.push(string_to_fingerprint("SOCK"));
    }
    fprts
}

pub(crate) fn load<F: Funge>(ctx: &mut InstructionContext<F>, fpr: i32) -> bool {
    if fpr == string_to_fingerprint("NULL") {
        NULL::load(ctx)
    } else if fpr == string_to_fingerprint("BOOL") {
        BOOL::load(ctx)
    } else if fpr == string_to_fingerprint("HRTI") {
        HRTI::load(ctx)
    } else if fpr == string_to_fingerprint("FIXP") {
        FIXP::load(ctx)
    } else if fpr == string_to_fingerprint("ROMA") {
        ROMA::load(ctx)
    } else if fpr == string_to_fingerprint("MODU") {
        MODU::load(ctx)
    } else if fpr == string_to_fingerprint("REFC") {
        REFC::load(ctx)
    } else if fpr == string_to_fingerprint("FPSP") {
        FPSP::load(ctx)
    } else if fpr == string_to_fingerprint("FPDP") {
        FPDP::load(ctx)
    } else if fpr == string_to_fingerprint("LONG") {
        LONG::load(ctx)
    } else if fpr == string_to_fingerprint("FPRT") {
        FPRT::load(ctx)
    } else if fpr == string_to_fingerprint("JSTR") {
        JSTR::load(ctx)
    } else if fpr == string_to_fingerprint("FRTH") {
        FRTH::load(ctx)
    } else if fpr == string_to_fingerprint("TURT") {
        TURT::load(ctx)
    } else {
        load_platform_specific(ctx, fpr)
    }
}

#[cfg(not(target_family = "wasm"))]
pub(crate) fn load_platform_specific<F: Funge>(ctx: &mut InstructionContext<F>, fpr: i32) -> bool {
    if fpr == string_to_fingerprint("SOCK") {
        SOCK::load(ctx)
    } else if fpr == string_to_fingerprint("TERM") {
        TERM::load(ctx)
    } else {
        false
    }
}

#[cfg(target_family = "wasm")]
pub(crate) fn load_platform_specific<F: Funge>(
    _ctx: &mut InstructionContext<F>,
    _fpr: i32,
) -> bool {
    false
}

pub(crate) fn unload<F: Funge>(ctx: &mut InstructionContext<F>, fpr: i32) -> bool {
    if fpr == string_to_fingerprint("NULL") {
        NULL::unload(ctx)
    } else if fpr == string_to_fingerprint("BOOL") {
        BOOL::unload(ctx)
    } else if fpr == string_to_fingerprint("HRTI") {
        HRTI::unload(ctx)
    } else if fpr == string_to_fingerprint("FIXP") {
        FIXP::unload(ctx)
    } else if fpr == string_to_fingerprint("ROMA") {
        ROMA::unload(ctx)
    } else if fpr == string_to_fingerprint("MODU") {
        MODU::unload(ctx)
    } else if fpr == string_to_fingerprint("REFC") {
        REFC::unload(ctx)
    } else if fpr == string_to_fingerprint("FPSP") {
        FPSP::unload(ctx)
    } else if fpr == string_to_fingerprint("FPDP") {
        FPDP::unload(ctx)
    } else if fpr == string_to_fingerprint("LONG") {
        LONG::unload(ctx)
    } else if fpr == string_to_fingerprint("FPRT") {
        FPRT::unload(ctx)
    } else if fpr == string_to_fingerprint("JSTR") {
        JSTR::unload(ctx)
    } else if fpr == string_to_fingerprint("FRTH") {
        FRTH::unload(ctx)
    } else if fpr == string_to_fingerprint("TURT") {
        TURT::unload(ctx)
    } else {
        unload_platform_specific(ctx, fpr)
    }
}

#[cfg(not(target_family = "wasm"))]
pub(crate) fn unload_platform_specific<F: Funge>(
    ctx: &mut InstructionContext<F>,
    fpr: i32,
) -> bool {
    if fpr == string_to_fingerprint("SOCK") {
        SOCK::unload(ctx)
    } else if fpr == string_to_fingerprint("TERM") {
        TERM::unload(ctx)
    } else {
        false
    }
}

#[cfg(target_family = "wasm")]
pub(crate) fn unload_platform_specific<F: Funge>(
    _ctx: &mut InstructionContext<F>,
    _fpr: i32,
) -> bool {
    false
}
