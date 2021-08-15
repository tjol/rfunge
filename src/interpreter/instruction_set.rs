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

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::io::{BufRead, BufReader, Read};

use num::ToPrimitive;
use unicode_reader::CodePoints;

use super::instructions;
use super::ip::InstructionPointer;
use super::motion::MotionCmds;
use super::{IOMode, InterpreterEnv};
use crate::fungespace::{FungeSpace, FungeValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionResult {
    Continue,
    StayPut,
    Skip,
    Exit,
    Panic,
}

type Instruction<Idx, Space, Env> =
    fn(&mut InstructionPointer<Idx, Space, Env>, &mut Space, &mut Env) -> InstructionResult;

type InstructionLayer<Idx, Space, Env> = Vec<Option<Instruction<Idx, Space, Env>>>;

#[derive(Debug, Clone, Copy)]
pub enum InstructionMode {
    Normal,
    String,
}

/// Struct encapulating the dynamic instructions loaded for an IP
/// It has multiple layers, and fingerprints are able to add a new
/// layer to the instruction set (which can later be popped)
#[derive(Clone)]
pub struct InstructionSet<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    pub mode: InstructionMode,
    layers: Vec<InstructionLayer<Idx, Space, Env>>,
}

impl<Idx, Space, Env> Debug for InstructionSet<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Function pointers don't implement Debug, so we need a work around
        write!(f, "<InstructionSet>")
    }
}

impl<Idx, Space, Env> InstructionSet<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    pub fn new() -> Self {
        let mut instruction_vec: InstructionLayer<Idx, Space, Env> = Vec::new();
        instruction_vec.resize(128, None);

        // Add standard instructions (other than those implemented directly
        // in the main match statement in exec_normal_instructions)
        instruction_vec['k' as usize] = Some(instructions::iterate);
        instruction_vec['{' as usize] = Some(instructions::begin_block);
        instruction_vec['}' as usize] = Some(instructions::end_block);
        instruction_vec['u' as usize] = Some(instructions::stack_under_stack);

        let mut layers = Vec::new();
        layers.push(instruction_vec);

        Self {
            mode: InstructionMode::Normal,
            layers: layers,
        }
    }

    /// Get the function associated with a given character, if any
    pub fn get_instruction(
        &self,
        instruction: Space::Output,
    ) -> Option<Instruction<Idx, Space, Env>> {
        *(self.layers[self.layers.len() - 1].get(instruction.to_usize()?)?)
    }

    /// Add a set of instructions as a new layer
    pub fn add_layer(&mut self, instructions: HashMap<u16, Instruction<Idx, Space, Env>>) {
        let mut new_layer = self.layers[self.layers.len() - 1].clone();
        for (&i, &f) in instructions.iter() {
            if i as usize >= new_layer.len() {
                new_layer.resize((i + 1) as usize, None);
            }
            new_layer[i as usize] = Some(f);
        }
        self.layers.push(new_layer);
    }

    pub fn pop_layer(&mut self) {
        self.layers.pop();
    }
}

pub fn exec_instruction<Idx, Space, Env>(
    raw_instruction: Space::Output,
    ip: &mut InstructionPointer<Idx, Space, Env>,
    space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    match ip.instructions.mode {
        InstructionMode::Normal => exec_normal_instruction(raw_instruction, ip, space, env),
        InstructionMode::String => exec_string_instruction(raw_instruction, ip, space, env),
    }
}

fn exec_normal_instruction<Idx, Space, Env>(
    raw_instruction: Space::Output,
    ip: &mut InstructionPointer<Idx, Space, Env>,
    space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    match raw_instruction.try_to_char() {
        Some('@') => InstructionResult::Exit,
        Some('#') => {
            // Trampoline
            ip.location = ip.location + ip.delta;
            InstructionResult::Continue
        }
        Some(';') => {
            loop {
                let (new_loc, new_val) = space.move_by(ip.location, ip.delta);
                ip.location = new_loc;
                if new_val.to_char() == ';' {
                    break;
                }
            }
            InstructionResult::Skip
        }
        Some('$') => {
            ip.pop();
            InstructionResult::Continue
        }
        Some('n') => {
            ip.stack_mut().drain(0..);
            InstructionResult::Continue
        }
        Some('\\') => {
            let a = ip.pop();
            let b = ip.pop();
            ip.push(a);
            ip.push(b);
            InstructionResult::Continue
        }
        Some(':') => {
            let n = ip.pop();
            ip.push(n);
            ip.push(n);
            InstructionResult::Continue
        }
        Some(digit) if digit >= '0' && digit <= '9' => {
            ip.push(((digit as i32) - ('0' as i32)).into());
            InstructionResult::Continue
        }
        Some(digit) if digit >= 'a' && digit <= 'f' => {
            ip.push((0xa + (digit as i32) - ('a' as i32)).into());
            InstructionResult::Continue
        }
        Some('"') => {
            ip.instructions.mode = InstructionMode::String;
            ip.location = ip.location + ip.delta;
            InstructionResult::StayPut
        }
        Some('\'') => {
            let loc = ip.location + ip.delta;
            ip.push(space[loc]);
            ip.location = loc;
            InstructionResult::Continue
        }
        Some('s') => {
            let loc = ip.location + ip.delta;
            space[loc] = ip.pop();
            ip.location = loc;
            InstructionResult::Continue
        }
        Some('.') => {
            if write!(env.output_writer(), "{} ", ip.pop()).is_err() {
                env.warn("IO Error");
            }
            InstructionResult::Continue
        }
        Some(',') => {
            let c = ip.pop();
            if match env.get_iomode() {
                IOMode::Text => write!(env.output_writer(), "{}", c.to_char()),
                IOMode::Binary => env
                    .output_writer()
                    .write(&[(c & 0xff.into()).to_u8().unwrap()])
                    .and_then(|_| Ok(())),
            }
            .is_err()
            {
                env.warn("IO Error");
            }
            InstructionResult::Continue
        }
        Some('~') => {
            match env.get_iomode() {
                IOMode::Binary => {
                    let mut buf = [0_u8; 1];
                    if matches!(env.input_reader().read(&mut buf), Ok(1)) {
                        ip.push((buf[0] as i32).into());
                    } else {
                        // reflect
                        ip.delta = ip.delta * (-1).into();
                    }
                }
                IOMode::Text => {
                    if let Some(Ok(c)) = CodePoints::from(env.input_reader().bytes()).next() {
                        ip.push((c as i32).into());
                    } else {
                        // reflect
                        ip.delta = ip.delta * (-1).into();
                    }
                }
            };
            InstructionResult::Continue
        }
        Some('&') => {
            let mut s = String::new();
            if BufReader::new(env.input_reader()).read_line(&mut s).is_ok() {
                let maybe_i: Result<i32, _> = s.trim().parse();
                if let Ok(i) = maybe_i {
                    ip.push(i.into());
                } else {
                    // reflect
                    ip.delta = ip.delta * (-1).into();
                }
            } else {
                // reflect
                ip.delta = ip.delta * (-1).into();
            }
            InstructionResult::Continue
        }
        Some('+') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(a + b);
            InstructionResult::Continue
        }
        Some('-') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(a - b);
            InstructionResult::Continue
        }
        Some('*') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(a * b);
            InstructionResult::Continue
        }
        Some('/') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(if b != 0.into() { a / b } else { 0.into() });
            InstructionResult::Continue
        }
        Some('%') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(if b != 0.into() { a % b } else { 0.into() });
            InstructionResult::Continue
        }
        Some('`') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(if a > b { 1.into() } else { 0.into() });
            InstructionResult::Continue
        }
        Some('!') => {
            let v = ip.pop();
            ip.push(if v == 0.into() { 1.into() } else { 0.into() });
            InstructionResult::Continue
        }
        Some('j') => {
            ip.location = ip.location + ip.delta * ip.pop();
            InstructionResult::Continue
        }
        Some('x') => {
            ip.delta = MotionCmds::pop_vector(ip);
            InstructionResult::Continue
        }
        Some('p') => {
            let loc = MotionCmds::pop_vector(ip) + ip.storage_offset;
            space[loc] = ip.pop();
            InstructionResult::Continue
        }
        Some('g') => {
            let loc = MotionCmds::pop_vector(ip) + ip.storage_offset;
            ip.push(space[loc]);
            InstructionResult::Continue
        }
        Some('r') => {
            ip.delta = ip.delta * (-1).into();
            InstructionResult::Continue
        }
        Some('z') => InstructionResult::Continue,
        Some(c) => {
            if MotionCmds::apply_delta(c, ip) {
                InstructionResult::Continue
            } else if let Some(instr_fn) = ip.instructions.get_instruction(raw_instruction) {
                (instr_fn)(ip, space, env)
            } else {
                // reflect
                ip.delta = ip.delta * (-1).into();
                env.warn(&format!("Unknown instruction: '{}'", c));
                InstructionResult::Continue
            }
        }
        None => {
            // reflect
            ip.delta = ip.delta * (-1).into();
            env.warn("Unknown non-Unicode instruction!");
            InstructionResult::Continue
        }
    }
}

fn exec_string_instruction<Idx, Space, Env>(
    raw_instruction: Space::Output,
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    match raw_instruction.to_char() {
        '"' => {
            ip.instructions.mode = InstructionMode::Normal;
            InstructionResult::Continue
        }
        ' ' => {
            ip.push(raw_instruction);
            // skip over the following spaces
            InstructionResult::Continue
        }
        _ => {
            // Some other character
            ip.push(raw_instruction);
            // Do not skip over the following spaces
            ip.location = ip.location + ip.delta;
            InstructionResult::StayPut
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::super::GenericEnv;
    use super::*;
    use crate::fungespace::index::BefungeVec;
    use crate::fungespace::paged::PagedFungeSpace;

    type SomeEnvType = GenericEnv<Box<dyn Read>, Box<dyn Write>, fn(&str)>;

    #[test]
    fn test_instruction_layers() {
        type Instr =
            Instruction<BefungeVec<i64>, PagedFungeSpace<BefungeVec<i64>, i64>, SomeEnvType>;
        let mut is = InstructionSet::<
            BefungeVec<i64>,
            PagedFungeSpace<BefungeVec<i64>, i64>,
            SomeEnvType,
        >::new();
        assert!(matches!(is.get_instruction('1' as i64), None));
        assert!(matches!(is.get_instruction('2' as i64), None));
        assert!(matches!(is.get_instruction('3' as i64), None));
        let mut new_layer = HashMap::new();
        new_layer.insert('2' as u16, nop_for_test as Instr);
        new_layer.insert('5' as u16, nop_for_test as Instr);
        is.add_layer(new_layer);
        assert!(matches!(is.get_instruction('1' as i64), None));
        assert!(matches!(is.get_instruction('2' as i64), Some(_)));
        assert!(matches!(is.get_instruction('3' as i64), None));
        is.pop_layer();
        assert!(matches!(is.get_instruction('1' as i64), None));
        assert!(matches!(is.get_instruction('2' as i64), None));
        assert!(matches!(is.get_instruction('3' as i64), None));
    }

    fn nop_for_test(
        _ip: &mut InstructionPointer<
            BefungeVec<i64>,
            PagedFungeSpace<BefungeVec<i64>, i64>,
            SomeEnvType,
        >,
        _sp: &mut PagedFungeSpace<BefungeVec<i64>, i64>,
        _env: &mut SomeEnvType,
    ) -> InstructionResult {
        InstructionResult::Continue
    }
}
