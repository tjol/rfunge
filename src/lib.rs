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

pub mod fungespace;
pub mod interpreter;

#[cfg(target_family = "wasm")]
mod wasm;

use std::hash::Hash;

use divrem::{DivEuclid, DivRemEuclid, RemEuclid};

pub use crate::fungespace::{
    bfvec, read_funge_src, read_funge_src_bin, BefungeVec, FungeSpace, FungeValue, PagedFungeSpace,
};
pub use crate::interpreter::{
    all_fingerprints, safe_fingerprints, string_to_fingerprint, ExecMode, Funge, IOMode,
    InstructionPointer, InstructionResult, Interpreter, InterpreterEnv, ProgramResult, RunMode,
};

/// Create a new Unefunge interpreter using the default implementation and
/// parameters.
///
/// `T` is the type of a unefunge cell (probably either `i32` or `i64`)
///
/// The environment, env, is where you pass IO functions and interpreter
/// settings.
///
/// After creating the interpreter, you can fill fungespace with
/// [read_funge_src] or [read_funge_src_bin].
pub fn new_unefunge_interpreter<T, Env>(env: Env) -> Interpreter<T, PagedFungeSpace<T, T>, Env>
where
    T: FungeValue + RemEuclid + Hash + DivEuclid + DivRemEuclid,
    Env: InterpreterEnv,
{
    Interpreter::new(PagedFungeSpace::new_with_page_size(1000.into()), env)
}

/// Create a new Unefunge interpreter using the default implementation and
/// parameters.
///
/// `T` is the type of a unefunge cell (probably either `i32` or `i64`)
///
/// The environment, env, is where you pass IO functions and interpreter
/// settings.
///
/// After creating the interpreter, you can fill fungespace with
/// [read_funge_src] or [read_funge_src_bin].
pub fn new_befunge_interpreter<T, Env>(
    env: Env,
) -> Interpreter<BefungeVec<T>, PagedFungeSpace<BefungeVec<T>, T>, Env>
where
    T: FungeValue + RemEuclid + Hash + DivEuclid + DivRemEuclid,
    Env: InterpreterEnv,
{
    Interpreter::new(PagedFungeSpace::new_with_page_size(bfvec(40, 20)), env)
}
