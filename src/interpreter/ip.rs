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

use std::any::Any;
use std::collections::HashMap;
use std::ops::Index;
use std::rc::Rc;

use super::instruction_set::InstructionSet;
use super::MotionCmds;
use crate::fungespace::index::{bfvec, BefungeVec};
use crate::fungespace::{FungeSpace, FungeValue};

/// Struct encapsulating the state of the/an IP
#[derive(Debug, Clone)]
pub struct InstructionPointer<Idx, Space>
where
    Idx: MotionCmds<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
{
    /// Location of the IP (initial: the origin)
    pub location: Idx,
    /// Current delta (initial: East)
    pub delta: Idx,
    /// Current storage offset (initial: the origin)
    pub storage_offset: Idx,
    /// The stack stack
    pub stack_stack: Vec<Vec<Space::Output>>,
    /// The currently available
    pub instructions: InstructionSet<Idx, Space>,
    /// If instructions or fingerprints need to store additional data with the
    /// IP, put them here.
    pub private_data: HashMap<String, Rc<dyn Any>>,
}

pub trait CreateInstructionPointer<Space>: MotionCmds<Space>
where
    Space: FungeSpace<Self>,
    Space::Output: FungeValue,
{
    fn new_ip() -> InstructionPointer<Self, Space>;
}

impl<T, Space> CreateInstructionPointer<Space> for T
where
    T: FungeValue,
    Space: FungeSpace<T> + Index<T, Output = T>,
{
    fn new_ip() -> InstructionPointer<T, Space> {
        let mut instance = InstructionPointer::<T, Space> {
            location: 0.into(),
            delta: 1.into(),
            storage_offset: 0.into(),
            stack_stack: Vec::new(),
            instructions: InstructionSet::new(),
            private_data: HashMap::new(),
        };
        instance.stack_stack.push(Vec::new());
        return instance;
    }
}

impl<T, Space> CreateInstructionPointer<Space> for BefungeVec<T>
where
    T: FungeValue,
    Space: FungeSpace<BefungeVec<T>, Output = T>,
{
    fn new_ip() -> InstructionPointer<BefungeVec<T>, Space> {
        let mut instance = InstructionPointer::<BefungeVec<T>, Space> {
            location: bfvec(0, 0),
            delta: bfvec(1, 0),
            storage_offset: bfvec(0, 0),
            stack_stack: Vec::new(),
            instructions: InstructionSet::new(),
            private_data: HashMap::new(),
        };
        instance.stack_stack.push(Vec::new());
        return instance;
    }
}

impl<Idx, Space> InstructionPointer<Idx, Space>
where
    Idx: CreateInstructionPointer<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
{
    pub fn new() -> Self {
        Idx::new_ip()
    }
}

impl<Idx, Space> InstructionPointer<Idx, Space>
where
    Idx: MotionCmds<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
{
    /// Get the top of the stack stack
    pub fn stack(&self) -> &Vec<Space::Output> {
        &self.stack_stack[self.stack_stack.len() - 1]
    }

    /// Get the top of the stack stack (mutable version)
    pub fn stack_mut(&mut self) -> &mut Vec<Space::Output> {
        let end = self.stack_stack.len() - 1;
        &mut self.stack_stack[end]
    }

    /// Pop one number from the stack and return it
    pub fn pop(&mut self) -> Space::Output {
        self.stack_mut().pop().unwrap_or(0.into())
    }

    /// Push a number onto the stack
    pub fn push(&mut self, v: Space::Output) {
        self.stack_mut().push(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fungespace::paged::PagedFungeSpace;

    #[test]
    fn test_stack() {
        let mut ip =
            InstructionPointer::<BefungeVec<i64>, PagedFungeSpace<BefungeVec<i64>, i64>>::new();

        assert_eq!(ip.pop(), 0);
        ip.push(1);
        ip.push(2);
        assert_eq!(ip.pop(), 2);
        ip.push(3);
        assert_eq!(ip.pop(), 3);
        assert_eq!(ip.pop(), 1);
        ip.push(4);
        ip.push(5);

        ip.stack_stack.push(Vec::new());
        assert_eq!(ip.pop(), 0);

        ip.stack_stack.pop();
        assert_eq!(ip.pop(), 5);
        assert_eq!(ip.stack().len(), 1);
    }
}