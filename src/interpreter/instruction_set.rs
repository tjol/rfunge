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

use std::cmp::{max, min};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::io::{BufRead, BufReader, Read};

use num::ToPrimitive;
use unicode_reader::CodePoints;

use super::ip::InstructionPointer;
use super::{IOMode, InterpreterEnv, MotionCmds};
use crate::fungespace::{FungeSpace, FungeValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionResult {
    Continue,
    StayPut,
    Skip,
    Exit,
    Panic,
}

// could use Rc<FnMut> instead of fn for more flexibility
type Instruction<Idx, Space> =
    fn(&mut InstructionPointer<Idx, Space>, &mut Space) -> InstructionResult;
type InstructionLayer<Idx, Space> = Vec<Option<Instruction<Idx, Space>>>;

#[derive(Debug, Clone, Copy)]
pub enum InstructionMode {
    Normal,
    String,
}

/// Struct encapulating the dynamic instructions loaded for an IP
/// It has multiple layers, and fingerprints are able to add a new
/// layer to the instruction set (which can later be popped)
#[derive(Clone)]
pub struct InstructionSet<Idx, Space>
where
    Idx: MotionCmds<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
{
    pub mode: InstructionMode,
    layers: Vec<InstructionLayer<Idx, Space>>,
}

impl<Idx, Space> Debug for InstructionSet<Idx, Space>
where
    Idx: MotionCmds<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Function pointers don't implement Debug, so we need a work around
        write!(f, "<InstructionSet>")
    }
}

impl<Idx, Space> InstructionSet<Idx, Space>
where
    Idx: MotionCmds<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
{
    pub fn new() -> Self {
        let mut instruction_vec = Vec::new();
        instruction_vec.resize(128, None);
        let mut layers = Vec::new();
        layers.push(instruction_vec);

        Self {
            mode: InstructionMode::Normal,
            layers: layers,
        }
    }

    /// Get the function associated with a given character, if any
    pub fn get_instruction(&self, instruction: Space::Output) -> Option<Instruction<Idx, Space>> {
        *(self.layers[self.layers.len() - 1].get(instruction.to_usize()?)?)
    }

    /// Add a set of instructions as a new layer
    pub fn add_layer(&mut self, instructions: HashMap<u16, Instruction<Idx, Space>>) {
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
    ip: &mut InstructionPointer<Idx, Space>,
    space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space>,
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
    ip: &mut InstructionPointer<Idx, Space>,
    space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space>,
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
        Some('k') => {
            let n = ip.pop();
            let (mut new_loc, new_val_ref) = space.move_by(ip.location, ip.delta);
            let mut new_val = *new_val_ref;
            let mut loop_result = InstructionResult::Continue;
            if let Some(n) = n.to_isize() {
                if n <= 0 {
                    // surprising behaviour! 1k leads to the next instruction
                    // being executed twice, 0k to it being skipped
                    ip.location = new_loc;
                    loop_result = InstructionResult::Continue;
                } else {
                    let mut new_val_c = new_val.to_char();
                    while new_val_c == ';' {
                        // skip what must be skipped
                        // fake-execute!
                        let old_loc = ip.location;
                        ip.location = new_loc;
                        exec_normal_instruction(new_val, ip, space, env);
                        let (new_loc2, new_val_ref) = space.move_by(ip.location, ip.delta);
                        new_loc = new_loc2;
                        new_val = *new_val_ref;
                        ip.location = old_loc;
                        new_val_c = new_val.to_char();
                    }
                    for _ in 0..n {
                        match exec_normal_instruction(new_val, ip, space, env) {
                            InstructionResult::Continue => {}
                            res => {
                                loop_result = res;
                                break;
                            }
                        }
                    }
                }
            } else {
                // Reflect on overflow
                ip.delta = ip.delta * (-1).into();
            }
            loop_result
        }
        Some('{') => {
            if let Some(n) = ip.pop().to_isize() {
                // take n items off the SOSS (old TOSS)
                let n_to_take = max(0, min(n, ip.stack().len() as isize));
                let zeros_for_toss = max(0, n - n_to_take);
                let zeros_for_soss = max(0, -n);

                let split_idx = ip.stack().len() - n_to_take as usize;
                let mut transfer_elems = ip.stack_mut().split_off(split_idx);

                for _ in 0..zeros_for_soss {
                    ip.push(0.into());
                }

                MotionCmds::push_vector(ip, ip.storage_offset); // onto SOSS / old TOSS

                // create a new stack
                ip.stack_stack.push(Vec::new());

                for _ in 0..zeros_for_toss {
                    ip.push(0.into());
                }

                ip.stack_mut().append(&mut transfer_elems);

                ip.storage_offset = ip.location + ip.delta;
            } else {
                // reflect
                ip.delta = ip.delta * (-1).into();
            }

            InstructionResult::Continue
        }
        Some('}') => {
            if ip.stack_stack.len() > 1 {
                if let Some(n) = ip.pop().to_isize() {
                    let mut toss = ip.stack_stack.pop().unwrap();

                    // restore the storage offset
                    ip.storage_offset = MotionCmds::pop_vector(ip);

                    let n_to_take = max(0, min(n, toss.len() as isize));
                    let zeros_for_soss = max(0, n - n_to_take);
                    let n_to_pop = max(0, -n);

                    if n_to_pop > 0 {
                        for _ in 0..n_to_pop {
                            ip.pop();
                        }
                    } else {
                        for _ in 0..zeros_for_soss {
                            ip.push(0.into());
                        }

                        let split_idx = toss.len() - n_to_take as usize;
                        ip.stack_mut().append(&mut toss.split_off(split_idx));
                    }
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
        Some('u') => {
            let nstacks = ip.stack_stack.len();
            if nstacks > 1 {
                if let Some(n) = ip.pop().to_isize() {
                    if n > 0 {
                        for _ in 0..n {
                            let v = ip.stack_stack[nstacks - 2].pop().unwrap_or(0.into());
                            ip.push(v);
                        }
                    } else if n < 0 {
                        for _ in 0..(-n) {
                            let v = ip.pop();
                            ip.stack_stack[nstacks - 2].push(v);
                        }
                    }
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
        Some('r') => {
            ip.delta = ip.delta * (-1).into();
            InstructionResult::Continue
        }
        Some('z') => InstructionResult::Continue,
        Some(c) => {
            if MotionCmds::apply_delta(c, ip) {
                InstructionResult::Continue
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
    ip: &mut InstructionPointer<Idx, Space>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space>,
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
    use super::*;
    use crate::fungespace::index::BefungeVec;
    use crate::fungespace::paged::PagedFungeSpace;

    #[test]
    fn test_instruction_layers() {
        type Instr = Instruction<BefungeVec<i64>, PagedFungeSpace<BefungeVec<i64>, i64>>;
        let mut is =
            InstructionSet::<BefungeVec<i64>, PagedFungeSpace<BefungeVec<i64>, i64>>::new();
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
        _ip: &mut InstructionPointer<BefungeVec<i64>, PagedFungeSpace<BefungeVec<i64>, i64>>,
        _sp: &mut PagedFungeSpace<BefungeVec<i64>, i64>,
    ) -> InstructionResult {
        InstructionResult::Continue
    }
}
