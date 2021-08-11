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
pub mod ip;

pub use crate::fungespace::{
    bfvec, read_befunge, read_befunge_bin, read_unefunge, read_unefunge_bin, BefungeVec,
    FungeSpace, PagedFungeSpace,
};
pub use crate::interpreter::{
    IOMode, InstructionResult, Interpreter, InterpreterEnvironment, ProgramResult,
};
pub use crate::ip::InstructionPointer;
