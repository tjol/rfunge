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
use std::fmt::{Debug, Formatter};
use std::ops::{Add, Index};
use std::rc::Rc;

use num::ToPrimitive;

use super::fungespace::index::{bfvec, BefungeVec};
use super::fungespace::{FungeIndex, FungeSpace, FungeValue};
use super::interpreter::InstructionResult;

/// Struct encapsulating the state of the/an IP
#[derive(Debug, Clone)]
pub struct InstructionPointer<Idx, Space>
where
    Idx: FungeIndex + Add<Output = Idx>,
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

pub trait CreateInstructionPointer<Space>: FungeIndex + Add<Output = Self>
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
    Idx: FungeIndex + Add<Output = Idx>,
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
    Idx: FungeIndex + Add<Output = Idx>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
{
    pub mode: InstructionMode,
    layers: Vec<InstructionLayer<Idx, Space>>,
}

impl<Idx, Space> Debug for InstructionSet<Idx, Space>
where
    Idx: FungeIndex + Add<Output = Idx>,
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
    Idx: FungeIndex + Add<Output = Idx>,
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

#[cfg(test)]
mod tests {
    use super::super::fungespace::paged::PagedFungeSpace;
    use super::*;

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
