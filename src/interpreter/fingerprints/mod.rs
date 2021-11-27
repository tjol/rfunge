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

#[cfg(all(feature = "ncurses", not(target_family = "wasm")))]
mod NCRS;

#[cfg(not(target_family = "wasm"))]
mod SOCK;

#[cfg(not(target_family = "wasm"))]
mod TERM;

use super::{Funge, InstructionPointer};

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
        if cfg!(feature = "ncurses") {
            fprts.push(string_to_fingerprint("NCRS"));
        }
    }
    fprts
}

pub(crate) fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
    fpr: i32,
) -> bool {
    if fpr == string_to_fingerprint("NULL") {
        NULL::load(ip, space, env)
    } else if fpr == string_to_fingerprint("BOOL") {
        BOOL::load(ip, space, env)
    } else if fpr == string_to_fingerprint("HRTI") {
        HRTI::load(ip, space, env)
    } else if fpr == string_to_fingerprint("FIXP") {
        FIXP::load(ip, space, env)
    } else if fpr == string_to_fingerprint("ROMA") {
        ROMA::load(ip, space, env)
    } else if fpr == string_to_fingerprint("MODU") {
        MODU::load(ip, space, env)
    } else if fpr == string_to_fingerprint("REFC") {
        REFC::load(ip, space, env)
    } else if fpr == string_to_fingerprint("FPSP") {
        FPSP::load(ip, space, env)
    } else if fpr == string_to_fingerprint("FPDP") {
        FPDP::load(ip, space, env)
    } else if fpr == string_to_fingerprint("LONG") {
        LONG::load(ip, space, env)
    } else if fpr == string_to_fingerprint("FPRT") {
        FPRT::load(ip, space, env)
    } else if fpr == string_to_fingerprint("JSTR") {
        JSTR::load(ip, space, env)
    } else if fpr == string_to_fingerprint("FRTH") {
        FRTH::load(ip, space, env)
    } else if fpr == string_to_fingerprint("TURT") {
        TURT::load(ip, space, env)
    } else {
        load_platform_specific(ip, space, env, fpr)
    }
}

#[cfg(not(target_family = "wasm"))]
pub(crate) fn load_platform_specific<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
    fpr: i32,
) -> bool {
    if fpr == string_to_fingerprint("SOCK") {
        SOCK::load(ip, space, env)
    } else if fpr == string_to_fingerprint("TERM") {
        TERM::load(ip, space, env)
    } else {
        maybe_load_ncrs(ip, space, env, fpr)
    }
}

#[cfg(all(feature = "ncurses", not(target_family = "wasm")))]
fn maybe_load_ncrs<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
    fpr: i32,
) -> bool {
    if fpr == string_to_fingerprint("NCRS") {
        NCRS::load(ip, space, env)
    } else {
        false
    }
}

#[cfg(not(any(feature = "ncurses", target_family = "wasm")))]
fn maybe_load_ncrs<F: Funge>(
    _ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
    _fpr: i32,
) -> bool {
    false
}

#[cfg(target_family = "wasm")]
pub(crate) fn load_platform_specific<F: Funge>(
    _ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
    _fpr: i32,
) -> bool {
    false
}

pub(crate) fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
    fpr: i32,
) -> bool {
    if fpr == string_to_fingerprint("NULL") {
        NULL::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("BOOL") {
        BOOL::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("HRTI") {
        HRTI::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("FIXP") {
        FIXP::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("ROMA") {
        ROMA::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("MODU") {
        MODU::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("REFC") {
        REFC::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("FPSP") {
        FPSP::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("FPDP") {
        FPDP::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("LONG") {
        LONG::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("FPRT") {
        FPRT::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("JSTR") {
        JSTR::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("FRTH") {
        FRTH::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("TURT") {
        TURT::unload(ip, space, env)
    } else {
        unload_platform_specific(ip, space, env, fpr)
    }
}

#[cfg(not(target_family = "wasm"))]
pub(crate) fn unload_platform_specific<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
    fpr: i32,
) -> bool {
    if fpr == string_to_fingerprint("SOCK") {
        SOCK::unload(ip, space, env)
    } else if fpr == string_to_fingerprint("TERM") {
        TERM::unload(ip, space, env)
    } else {
        maybe_unload_ncrs(ip, space, env, fpr)
    }
}

#[cfg(all(feature = "ncurses", not(target_family = "wasm")))]
fn maybe_unload_ncrs<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
    fpr: i32,
) -> bool {
    if fpr == string_to_fingerprint("NCRS") {
        NCRS::unload(ip, space, env)
    } else {
        false
    }
}

#[cfg(not(any(feature = "ncurses", target_family = "wasm")))]
fn maybe_unload_ncrs<F: Funge>(
    _ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
    _fpr: i32,
) -> bool {
    false
}

#[cfg(target_family = "wasm")]
pub(crate) fn unload_platform_specific<F: Funge>(
    _ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
    _fpr: i32,
) -> bool {
    false
}
