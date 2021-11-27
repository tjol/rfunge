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
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
// use std::rc::Rc;
use std::str;

use futures_lite::io::{AsyncReadExt, AsyncWriteExt};
use num::ToPrimitive;

use super::fingerprints;
use super::instructions;
use super::ip::InstructionPointer;
use super::motion::MotionCmds;
use super::{Funge, IOMode, InterpreterEnv};
use crate::fungespace::{FungeSpace, FungeValue};

/// Result of a single instruction. Most instructions return
/// [InstructionResult::Continue].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionResult {
    /// Continue processing
    Continue,
    /// Continue processing within the same tick (only used by `;`)
    Skip,
    /// Spawn new IPs (only used by `t`... and `kt`)
    Fork(i32),
    /// Stop this IP (only used by `@`)
    Stop,
    /// Exit the program with a supplied code (only used by `q`)
    Exit(i32),
    /// Abort/panic. Do not use if it can be at all avoided.
    Panic,
}

pub enum Instruction<F: Funge + 'static> {
    SyncInstruction(SyncInstructionPtr<F>),
    AsyncInstruction(AsyncInstructionPtr<F>),
}

pub type SyncInstructionPtr<F> = fn(
    &mut InstructionPointer<F>,
    &mut <F as Funge>::Space,
    &mut <F as Funge>::Env,
) -> InstructionResult;

pub type AsyncInstructionPtr<F> =
    for<'a> fn(
        &'a mut InstructionPointer<F>,
        &'a mut <F as Funge>::Space,
        &'a mut <F as Funge>::Env,
    ) -> Pin<Box<dyn Future<Output = InstructionResult> + 'a>>;

impl<F: Funge + 'static> Clone for Instruction<F> {
    fn clone(&self) -> Self {
        match self {
            Instruction::SyncInstruction(f) => Instruction::SyncInstruction(*f),
            Instruction::AsyncInstruction(af) => Instruction::AsyncInstruction(*af),
        }
    }
}

/// Turn a regular fuction into an `Instruction`
pub fn sync_instruction<F: Funge + 'static>(func: SyncInstructionPtr<F>) -> Instruction<F>
where
    F: Funge + 'static,
{
    Instruction::SyncInstruction(func)
}

#[derive(Debug, Clone, Copy)]
pub enum InstructionMode {
    Normal,
    String,
}

/// Struct encapulating the dynamic instructions loaded for an IP
/// It has multiple layers, and fingerprints are able to add a new
/// layer to the instruction set (which can later be popped)
pub struct InstructionSet<F: Funge + 'static> {
    pub mode: InstructionMode,
    instructions: Vec<Vec<Instruction<F>>>,
}

// Can't derive Clone by macro because it requires the type parameters to be
// Clone...
impl<F: Funge + 'static> Clone for InstructionSet<F> {
    fn clone(&self) -> Self {
        Self {
            mode: self.mode,
            instructions: self.instructions.clone(),
        }
    }
}

// Can't derive Debug by macro because of the function pointers
impl<F: Funge + 'static> Debug for InstructionSet<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Function pointers don't implement Debug, so we need a work around
        write!(f, "<InstructionSet>")
    }
}

impl<F: Funge + 'static> Default for InstructionSet<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Funge + 'static> InstructionSet<F> {
    /// Create a new [InstructionSet] with the default commands
    pub fn new() -> Self {
        let mut instruction_vec: Vec<Vec<Instruction<F>>> = Vec::new();
        instruction_vec.resize_with(128, Vec::new);

        // Add standard instructions (other than those implemented directly
        // in the main match statement in exec_normal_instructions)
        instruction_vec['k' as usize].push(Instruction::AsyncInstruction(instructions::iterate));
        instruction_vec['{' as usize].push(sync_instruction(instructions::begin_block));
        instruction_vec['}' as usize].push(sync_instruction(instructions::end_block));
        instruction_vec['u' as usize].push(sync_instruction(instructions::stack_under_stack));
        instruction_vec['i' as usize].push(sync_instruction(instructions::input_file));
        instruction_vec['o' as usize].push(sync_instruction(instructions::output_file));
        instruction_vec['=' as usize].push(sync_instruction(instructions::execute));
        instruction_vec['y' as usize].push(sync_instruction(instructions::sysinfo));

        Self {
            mode: InstructionMode::Normal,
            instructions: instruction_vec,
        }
    }

    /// Get the function associated with a given character, if any
    pub fn get_instruction(&self, instruction: F::Value) -> Option<Instruction<F>> {
        let instr_stack = self.instructions.get(instruction.to_usize()?)?;
        if !instr_stack.is_empty() {
            Some(instr_stack[instr_stack.len() - 1].clone())
        } else {
            None
        }
    }

    /// Add a set of instructions as a new layer
    pub fn add_layer(&mut self, instructions: HashMap<char, Instruction<F>>) {
        for (&i, f) in instructions.iter() {
            if i as usize >= self.instructions.len() {
                self.instructions.resize_with((i as usize) + 1, Vec::new);
            }
            self.instructions[i as usize].push(f.clone());
        }
    }

    /// Remove the top layer for given instructions
    pub fn pop_layer(&mut self, instructions: &[char]) -> bool {
        let mut any_popped = false;
        for c in instructions {
            let i = *c as usize;
            if i < self.instructions.len() && !self.instructions[i].is_empty() {
                self.instructions[i].pop();
                any_popped = true;
            }
        }
        any_popped
    }
}

#[inline]
pub(super) async fn exec_instruction<'a, F: Funge + 'static>(
    raw_instruction: F::Value,
    ip: &'a mut InstructionPointer<F>,
    space: &'a mut F::Space,
    env: &'a mut F::Env,
) -> InstructionResult {
    match ip.instructions.mode {
        InstructionMode::Normal => exec_normal_instruction(raw_instruction, ip, space, env).await,
        InstructionMode::String => exec_string_instruction(raw_instruction, ip, space, env).await,
    }
}

#[inline]
async fn exec_normal_instruction<'a, F: Funge + 'static>(
    raw_instruction: F::Value,
    ip: &'a mut InstructionPointer<F>,
    space: &'a mut F::Space,
    env: &'a mut F::Env,
) -> InstructionResult {
    match raw_instruction.try_to_char() {
        Some(' ') => {
            return InstructionResult::Skip;
        }
        Some('@') => {
            return InstructionResult::Stop;
        }
        Some('t') => {
            return InstructionResult::Fork(1);
        }
        Some('q') => {
            let res = InstructionResult::Exit(ip.pop().to_i32().unwrap_or(-1));
            return res;
        }
        Some('#') => {
            // Trampoline
            ip.location = ip.location + ip.delta;
        }
        Some(';') => {
            loop {
                let (new_loc, new_val) = space.move_by(ip.location, ip.delta);
                ip.location = new_loc;
                if new_val.to_char() == ';' {
                    break;
                }
            }
            return InstructionResult::Skip;
        }
        Some('$') => {
            ip.pop();
        }
        Some('n') => {
            ip.stack_mut().drain(0..);
        }
        Some('\\') => {
            let a = ip.pop();
            let b = ip.pop();
            ip.push(a);
            ip.push(b);
        }
        Some(':') => {
            let n = ip.pop();
            ip.push(n);
            ip.push(n);
        }
        Some(digit) if ('0'..='9').contains(&digit) => {
            ip.push(((digit as i32) - ('0' as i32)).into());
        }
        Some(digit) if ('a'..='f').contains(&digit) => {
            ip.push((0xa + (digit as i32) - ('a' as i32)).into());
        }
        Some('"') => {
            ip.instructions.mode = InstructionMode::String;
        }
        Some('\'') => {
            let loc = ip.location + ip.delta;
            ip.push(space[loc]);
            ip.location = loc;
        }
        Some('s') => {
            let loc = ip.location + ip.delta;
            space[loc] = ip.pop();
            ip.location = loc;
        }
        Some('.') => {
            let s = format!("{} ", ip.pop());
            if env.output_writer().write(s.as_bytes()).await.is_err() {
                env.warn("IO Error");
            }
        }
        Some(',') => {
            let c = ip.pop();
            let buf = match env.get_iomode() {
                IOMode::Text => format!("{}", c.to_char()).into_bytes(),
                IOMode::Binary => vec![(c & 0xff.into()).to_u8().unwrap()],
            };
            if env.output_writer().write(&buf).await.is_err() {
                env.warn("IO Error");
            }
        }
        Some('~') => {
            match env.get_iomode() {
                IOMode::Binary => {
                    let mut buf = [0_u8; 1];
                    match env.input_reader().read(&mut buf).await {
                        Ok(1) => ip.push((buf[0] as i32).into()),
                        _ => ip.reflect(),
                    }
                }
                IOMode::Text => {
                    // Read bytes and decode
                    let mut buf = Vec::new();
                    let reader = env.input_reader();
                    loop {
                        let idx = buf.len();
                        buf.push(0_u8);
                        match reader.read(&mut buf[idx..]).await {
                            Ok(1) => {
                                // Try to decode
                                match str::from_utf8(&buf) {
                                    Ok(s) => {
                                        // Good!
                                        let c = s.chars().next().unwrap();
                                        ip.push((c as i32).into());
                                        break;
                                    }
                                    Err(err) => {
                                        match err.error_len() {
                                            None => {
                                                // more to come
                                            }
                                            Some(_) => {
                                                // Invalid
                                                ip.reflect();
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Read error
                                ip.reflect();
                                break;
                            }
                        }
                    }
                }
            };
        }
        Some('&') => {
            let mut buf = Vec::new();
            let reader = env.input_reader();
            let mut maybe_line = None;
            loop {
                let idx = buf.len();
                buf.push(0_u8);
                match reader.read(&mut buf[idx..]).await {
                    Ok(1) => {
                        if buf[idx] == b'\n' {
                            // End of line
                            maybe_line = str::from_utf8(&buf).ok();
                            break;
                        }
                    }
                    _ => {
                        // error
                        break;
                    }
                }
            }
            if let Some(line) = maybe_line {
                let maybe_i: Result<i32, _> = line.trim().parse();
                if let Ok(i) = maybe_i {
                    ip.push(i.into());
                } else {
                    ip.reflect();
                }
            } else {
                ip.reflect();
            }
        }
        Some('+') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(a + b);
        }
        Some('-') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(a - b);
        }
        Some('*') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(a * b);
        }
        Some('/') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(if b != 0.into() { a / b } else { 0.into() });
        }
        Some('%') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(if b != 0.into() { a % b } else { 0.into() });
        }
        Some('`') => {
            let b = ip.pop();
            let a = ip.pop();
            ip.push(if a > b { 1.into() } else { 0.into() });
        }
        Some('!') => {
            let v = ip.pop();
            ip.push(if v == 0.into() { 1.into() } else { 0.into() });
        }
        Some('j') => {
            ip.location = ip.location + ip.delta * ip.pop();
        }
        Some('x') => {
            ip.delta = MotionCmds::pop_vector(ip);
        }
        Some('p') => {
            let loc = MotionCmds::pop_vector(ip) + ip.storage_offset;
            space[loc] = ip.pop();
        }
        Some('g') => {
            let loc = MotionCmds::pop_vector(ip) + ip.storage_offset;
            ip.push(space[loc]);
        }
        Some('(') => {
            let count = ip.pop().to_usize().unwrap_or(0);
            let mut fpr = 0;
            for _ in 0..count {
                fpr <<= 8;
                fpr += ip.pop().to_i32().unwrap_or(0);
            }
            if fpr != 0 && env.is_fingerprint_enabled(fpr) {
                if fingerprints::load(ip, space, env, fpr) {
                    ip.push(fpr.into());
                    ip.push(1.into());
                } else {
                    ip.reflect();
                }
            } else {
                ip.reflect();
            }
        }
        Some(')') => {
            let count = ip.pop().to_usize().unwrap_or(0);
            let mut fpr = 0;
            for _ in 0..count {
                fpr <<= 8;
                fpr += ip.pop().to_i32().unwrap_or(0);
            }
            if fpr != 0 {
                if fingerprints::unload(ip, space, env, fpr) {
                    ip.push(fpr.into());
                    ip.push(1.into());
                } else {
                    ip.reflect();
                }
            } else {
                ip.reflect();
            }
        }
        Some('r') => {
            ip.reflect();
        }
        Some('z') => {}
        Some(c) => {
            if MotionCmds::apply_delta(c, ip) {
                // ok
            } else if let Some(instr) = ip.instructions.get_instruction(raw_instruction) {
                // return (instr_fn)(ctx).await;
                return match instr {
                    Instruction::SyncInstruction(func) => func(ip, space, env),
                    Instruction::AsyncInstruction(async_func) => (async_func)(ip, space, env).await,
                };
            } else {
                ip.reflect();
                env.warn(&format!("Unknown instruction: '{}'", c));
            }
        }
        None => {
            ip.reflect();
            env.warn("Unknown non-Unicode instruction!");
        }
    }
    InstructionResult::Continue
}

#[inline]
async fn exec_string_instruction<F: Funge + 'static>(
    raw_instruction: F::Value,
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    // did we just skip over a space?
    let prev_loc = ip.location - ip.delta;
    let prev_val = space[prev_loc];
    if prev_val == (' ' as i32).into() {
        ip.push(prev_val);
    }
    match raw_instruction.to_char() {
        '"' => {
            ip.instructions.mode = InstructionMode::Normal;
        }
        _ => {
            // Push this character.
            ip.push(raw_instruction);
        }
    }
    InstructionResult::Continue
}

#[cfg(test)]
mod tests {
    use super::super::tests::TestFunge;
    use super::*;

    #[test]
    fn test_instruction_layers() {
        let mut is = InstructionSet::<TestFunge>::new();
        assert!(matches!(is.get_instruction('1' as i64), None));
        assert!(matches!(is.get_instruction('2' as i64), None));
        assert!(matches!(is.get_instruction('3' as i64), None));
        let mut new_layer = HashMap::new();
        new_layer.insert('2', sync_instruction(nop_for_test));
        new_layer.insert('5', sync_instruction(nop_for_test));
        is.add_layer(new_layer);
        assert!(matches!(is.get_instruction('1' as i64), None));
        assert!(matches!(is.get_instruction('2' as i64), Some(_)));
        assert!(matches!(is.get_instruction('3' as i64), None));
        is.pop_layer(&['2', '5']);
        assert!(matches!(is.get_instruction('1' as i64), None));
        assert!(matches!(is.get_instruction('2' as i64), None));
        assert!(matches!(is.get_instruction('3' as i64), None));
    }

    fn nop_for_test(
        _ip: &mut InstructionPointer<TestFunge>,
        _space: &mut <TestFunge as Funge>::Space,
        _env: &mut <TestFunge as Funge>::Env,
    ) -> InstructionResult {
        InstructionResult::Continue
    }
}
