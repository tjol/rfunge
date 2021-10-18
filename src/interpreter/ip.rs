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
use std::any::Any;
use std::ops::Index;
use std::rc::Rc;

use super::instruction_set::InstructionSet;
use super::motion::MotionCmds;
use super::InterpreterEnv;
use crate::fungespace::index::{bfvec, BefungeVec};
use crate::fungespace::{FungeSpace, FungeValue, SrcIO};

/// Struct encapsulating the state of the/an IP
#[derive(Debug)]
pub struct InstructionPointer<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    /// Identifier of the IP
    pub id: Space::Output,
    /// Location of the IP (initial: the origin)
    pub location: Idx,
    /// Current delta (initial: East)
    pub delta: Idx,
    /// Current storage offset (initial: the origin)
    pub storage_offset: Idx,
    /// The stack stack
    pub stack_stack: Vec<Vec<Space::Output>>,
    /// The currently available
    pub instructions: InstructionSet<Idx, Space, Env>,
    /// If instructions or fingerprints need to store additional data with the
    /// IP, put them here.
    pub private_data: HashMap<String, Rc<dyn Any>>,
}

// Can't derive Clone by macro because it requires the type parameters to be
// Clone...
impl<Idx, Space, Env> Clone for InstructionPointer<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            location: self.location,
            delta: self.delta,
            storage_offset: self.storage_offset,
            stack_stack: self.stack_stack.clone(),
            instructions: self.instructions.clone(),
            private_data: self.private_data.clone(),
        }
    }
}

/// Helper trait used by [InstructionPointer::new]
pub trait CreateInstructionPointer<Space, Env>: MotionCmds<Space, Env> + SrcIO<Space>
where
    Space: FungeSpace<Self>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    /// Create a new IP with the appropriate initial location, delta,
    /// storage offset, etc.
    fn new_ip() -> InstructionPointer<Self, Space, Env>;
}

impl<T, Space, Env> CreateInstructionPointer<Space, Env> for T
where
    T: FungeValue,
    Space: FungeSpace<T> + Index<T, Output = T>,
    Env: InterpreterEnv,
{
    fn new_ip() -> InstructionPointer<T, Space, Env> {
        InstructionPointer {
            id: 0.into(),
            location: (-1).into(),
            delta: 1.into(),
            storage_offset: 0.into(),
            stack_stack: vec![Vec::new()],
            instructions: InstructionSet::new(),
            private_data: HashMap::new(),
        }
    }
}

impl<T, Space, Env> CreateInstructionPointer<Space, Env> for BefungeVec<T>
where
    T: FungeValue,
    Space: FungeSpace<BefungeVec<T>, Output = T>,
    Env: InterpreterEnv,
{
    fn new_ip() -> InstructionPointer<BefungeVec<T>, Space, Env> {
        InstructionPointer {
            id: 0.into(),
            location: bfvec(-1, 0),
            delta: bfvec(1, 0),
            storage_offset: bfvec(0, 0),
            stack_stack: vec![Vec::new()],
            instructions: InstructionSet::new(),
            private_data: HashMap::new(),
        }
    }
}

impl<Idx, Space, Env> Default for InstructionPointer<Idx, Space, Env>
where
    Idx: CreateInstructionPointer<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Idx, Space, Env> InstructionPointer<Idx, Space, Env>
where
    Idx: CreateInstructionPointer<Space, Env>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    pub fn new() -> Self {
        Idx::new_ip()
    }
}

impl<Idx, Space, Env> InstructionPointer<Idx, Space, Env>
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    /// Get the top of the stack stack
    #[inline]
    pub fn stack(&self) -> &Vec<Space::Output> {
        &self.stack_stack[0]
    }

    /// Get the top of the stack stack (mutable version)
    #[inline]
    pub fn stack_mut(&mut self) -> &mut Vec<Space::Output> {
        &mut self.stack_stack[0]
    }

    /// Pop one number from the stack and return it
    #[inline]
    pub fn pop(&mut self) -> Space::Output {
        self.stack_mut().pop().unwrap_or_else(|| 0.into())
    }

    /// Push a number onto the stack
    #[inline]
    pub fn push(&mut self, v: Space::Output) {
        self.stack_mut().push(v)
    }

    /// Pop a 0gnirts off the stack as a string
    pub fn pop_0gnirts(&mut self) -> String {
        let mut c = self.pop();
        let mut s = String::new();
        while c != 0.into() {
            s.push(c.to_char());
            c = self.pop();
        }
        s
    }

    /// Push a string onto the stack as a 0gnirts
    pub fn push_0gnirts(&mut self, s: &str) {
        self.push(0.into());
        for c in s.chars().rev() {
            self.push((c as i32).into());
        }
    }

    /// Reflect the IP
    #[inline]
    pub fn reflect(&mut self) {
        self.delta = self.delta * (-1).into();
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::NoEnv;
    use super::*;
    use crate::fungespace::paged::PagedFungeSpace;

    #[test]
    fn test_stack() {
        let mut ip = InstructionPointer::<
            BefungeVec<i64>,
            PagedFungeSpace<BefungeVec<i64>, i64>,
            NoEnv,
        >::new();

        assert_eq!(ip.pop(), 0);
        ip.push(1);
        ip.push(2);
        assert_eq!(ip.pop(), 2);
        ip.push(3);
        assert_eq!(ip.pop(), 3);
        assert_eq!(ip.pop(), 1);
        ip.push(4);
        ip.push(5);

        ip.stack_stack.insert(0, Vec::new());
        assert_eq!(ip.pop(), 0);

        ip.stack_stack.remove(0);
        assert_eq!(ip.pop(), 5);
        assert_eq!(ip.stack().len(), 1);
    }
}
