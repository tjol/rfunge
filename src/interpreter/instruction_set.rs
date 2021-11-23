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

/// State of the interpreter. An (async) instruction takes ownership of this
/// while it is being executed.
pub struct InstructionContext<F: Funge + 'static> {
    pub ip: Box<InstructionPointer<F>>,
    pub space: Box<F::Space>,
    pub env: Box<F::Env>,
}

pub enum Instruction<F: Funge + 'static> {
    SyncInstruction(fn(&mut InstructionContext<F>) -> InstructionResult),
    AsyncInstruction(AsyncInstructionPtr<F>),
}

// pub type AsyncInstructionPtr<F> = Rc<
//     dyn for<'a> Fn(
//         &'a mut InstructionContext<F>,
//     ) -> Pin<Box<dyn Future<Output = InstructionResult> + 'a>>,
// >;
pub type AsyncInstructionPtr<F> =
    for<'a> fn(
        &'a mut InstructionContext<F>,
    ) -> Pin<Box<dyn Future<Output = InstructionResult> + 'a>>;

impl<F: Funge + 'static> Clone for Instruction<F> {
    fn clone(&self) -> Self {
        match self {
            Instruction::SyncInstruction(f) => Instruction::SyncInstruction(*f),
            Instruction::AsyncInstruction(af) => Instruction::AsyncInstruction(af.clone()),
        }
    }
}

/// Turn a regular fuction into an `Instruction`
pub fn sync_instruction<F: Funge + 'static>(
    func: fn(&mut InstructionContext<F>) -> InstructionResult,
) -> Instruction<F>
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
pub(super) async fn exec_instruction<F: Funge + 'static>(
    raw_instruction: F::Value,
    ctx: &mut InstructionContext<F>,
) -> InstructionResult {
    match ctx.ip.instructions.mode {
        InstructionMode::Normal => exec_normal_instruction(raw_instruction, ctx).await,
        InstructionMode::String => exec_string_instruction(raw_instruction, ctx).await,
    }
}

#[inline]
async fn exec_normal_instruction<F: Funge + 'static>(
    raw_instruction: F::Value,
    ctx: &mut InstructionContext<F>,
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
            let res = InstructionResult::Exit(ctx.ip.pop().to_i32().unwrap_or(-1));
            return res;
        }
        Some('#') => {
            // Trampoline
            ctx.ip.location = ctx.ip.location + ctx.ip.delta;
        }
        Some(';') => {
            loop {
                let (new_loc, new_val) = ctx.space.move_by(ctx.ip.location, ctx.ip.delta);
                ctx.ip.location = new_loc;
                if new_val.to_char() == ';' {
                    break;
                }
            }
            return InstructionResult::Skip;
        }
        Some('$') => {
            ctx.ip.pop();
        }
        Some('n') => {
            ctx.ip.stack_mut().drain(0..);
        }
        Some('\\') => {
            let a = ctx.ip.pop();
            let b = ctx.ip.pop();
            ctx.ip.push(a);
            ctx.ip.push(b);
        }
        Some(':') => {
            let n = ctx.ip.pop();
            ctx.ip.push(n);
            ctx.ip.push(n);
        }
        Some(digit) if ('0'..='9').contains(&digit) => {
            ctx.ip.push(((digit as i32) - ('0' as i32)).into());
        }
        Some(digit) if ('a'..='f').contains(&digit) => {
            ctx.ip.push((0xa + (digit as i32) - ('a' as i32)).into());
        }
        Some('"') => {
            ctx.ip.instructions.mode = InstructionMode::String;
        }
        Some('\'') => {
            let loc = ctx.ip.location + ctx.ip.delta;
            ctx.ip.push(ctx.space[loc]);
            ctx.ip.location = loc;
        }
        Some('s') => {
            let loc = ctx.ip.location + ctx.ip.delta;
            ctx.space[loc] = ctx.ip.pop();
            ctx.ip.location = loc;
        }
        Some('.') => {
            let s = format!("{} ", ctx.ip.pop());
            if ctx.env.output_writer().write(s.as_bytes()).await.is_err() {
                ctx.env.warn("IO Error");
            }
        }
        Some(',') => {
            let c = ctx.ip.pop();
            let buf = match ctx.env.get_iomode() {
                IOMode::Text => format!("{}", c.to_char()).into_bytes(),
                IOMode::Binary => vec![(c & 0xff.into()).to_u8().unwrap()],
            };
            if ctx.env.output_writer().write(&buf).await.is_err() {
                ctx.env.warn("IO Error");
            }
        }
        Some('~') => {
            match ctx.env.get_iomode() {
                IOMode::Binary => {
                    let mut buf = [0_u8; 1];
                    match ctx.env.input_reader().read(&mut buf).await {
                        Ok(1) => ctx.ip.push((buf[0] as i32).into()),
                        _ => ctx.ip.reflect(),
                    }
                }
                IOMode::Text => {
                    // Read bytes and decode
                    let mut buf = Vec::new();
                    let reader = ctx.env.input_reader();
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
                                        ctx.ip.push((c as i32).into());
                                        break;
                                    }
                                    Err(err) => {
                                        match err.error_len() {
                                            None => {
                                                // more to come
                                            }
                                            Some(_) => {
                                                // Invalid
                                                ctx.ip.reflect();
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Read error
                                ctx.ip.reflect();
                                break;
                            }
                        }
                    }
                }
            };
        }
        Some('&') => {
            let mut buf = Vec::new();
            let reader = ctx.env.input_reader();
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
                    ctx.ip.push(i.into());
                } else {
                    ctx.ip.reflect();
                }
            } else {
                ctx.ip.reflect();
            }
        }
        Some('+') => {
            let b = ctx.ip.pop();
            let a = ctx.ip.pop();
            ctx.ip.push(a + b);
        }
        Some('-') => {
            let b = ctx.ip.pop();
            let a = ctx.ip.pop();
            ctx.ip.push(a - b);
        }
        Some('*') => {
            let b = ctx.ip.pop();
            let a = ctx.ip.pop();
            ctx.ip.push(a * b);
        }
        Some('/') => {
            let b = ctx.ip.pop();
            let a = ctx.ip.pop();
            ctx.ip.push(if b != 0.into() { a / b } else { 0.into() });
        }
        Some('%') => {
            let b = ctx.ip.pop();
            let a = ctx.ip.pop();
            ctx.ip.push(if b != 0.into() { a % b } else { 0.into() });
        }
        Some('`') => {
            let b = ctx.ip.pop();
            let a = ctx.ip.pop();
            ctx.ip.push(if a > b { 1.into() } else { 0.into() });
        }
        Some('!') => {
            let v = ctx.ip.pop();
            ctx.ip.push(if v == 0.into() { 1.into() } else { 0.into() });
        }
        Some('j') => {
            ctx.ip.location = ctx.ip.location + ctx.ip.delta * ctx.ip.pop();
        }
        Some('x') => {
            ctx.ip.delta = MotionCmds::pop_vector(&mut ctx.ip);
        }
        Some('p') => {
            let loc = MotionCmds::pop_vector(&mut ctx.ip) + ctx.ip.storage_offset;
            ctx.space[loc] = ctx.ip.pop();
        }
        Some('g') => {
            let loc = MotionCmds::pop_vector(&mut ctx.ip) + ctx.ip.storage_offset;
            ctx.ip.push(ctx.space[loc]);
        }
        Some('(') => {
            let count = ctx.ip.pop().to_usize().unwrap_or(0);
            let mut fpr = 0;
            for _ in 0..count {
                fpr <<= 8;
                fpr += ctx.ip.pop().to_i32().unwrap_or(0);
            }
            if fpr != 0 && ctx.env.is_fingerprint_enabled(fpr) {
                if fingerprints::load(ctx, fpr) {
                    ctx.ip.push(fpr.into());
                    ctx.ip.push(1.into());
                } else {
                    ctx.ip.reflect();
                }
            } else {
                ctx.ip.reflect();
            }
        }
        Some(')') => {
            let count = ctx.ip.pop().to_usize().unwrap_or(0);
            let mut fpr = 0;
            for _ in 0..count {
                fpr <<= 8;
                fpr += ctx.ip.pop().to_i32().unwrap_or(0);
            }
            if fpr != 0 {
                if fingerprints::unload(ctx, fpr) {
                    ctx.ip.push(fpr.into());
                    ctx.ip.push(1.into());
                } else {
                    ctx.ip.reflect();
                }
            } else {
                ctx.ip.reflect();
            }
        }
        Some('r') => {
            ctx.ip.reflect();
        }
        Some('z') => {}
        Some(c) => {
            if MotionCmds::apply_delta(c, &mut ctx.ip) {
                // ok
            } else if let Some(instr) = ctx.ip.instructions.get_instruction(raw_instruction) {
                // return (instr_fn)(ctx).await;
                return match instr {
                    Instruction::SyncInstruction(func) => func(ctx),
                    Instruction::AsyncInstruction(async_func) => (async_func)(ctx).await,
                };
            } else {
                ctx.ip.reflect();
                ctx.env.warn(&format!("Unknown instruction: '{}'", c));
            }
        }
        None => {
            ctx.ip.reflect();
            ctx.env.warn("Unknown non-Unicode instruction!");
        }
    }
    InstructionResult::Continue
}

#[inline]
async fn exec_string_instruction<F: Funge + 'static>(
    raw_instruction: F::Value,
    ctx: &mut InstructionContext<F>,
) -> InstructionResult {
    // did we just skip over a space?
    let prev_loc = ctx.ip.location - ctx.ip.delta;
    let prev_val = ctx.space[prev_loc];
    if prev_val == (' ' as i32).into() {
        ctx.ip.push(prev_val);
    }
    match raw_instruction.to_char() {
        '"' => {
            ctx.ip.instructions.mode = InstructionMode::Normal;
        }
        _ => {
            // Push this character.
            ctx.ip.push(raw_instruction);
        }
    }
    InstructionResult::Continue
}

#[cfg(test)]
mod tests {
    use super::super::tests::TestFunge;
    use super::*;

    type TestCtx = InstructionContext<TestFunge>;

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

    fn nop_for_test(_ctx: &mut TestCtx) -> InstructionResult {
        InstructionResult::Continue
    }
}
