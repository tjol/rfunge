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

use super::fungespace::FungeIndex;
use super::{bfvec, BefungeVec64, FungeSpace};
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

use num::Integer;
use std::collections::LinkedList;
use std::ops::Add;

#[derive(Debug, Clone)]
pub struct InstructionPointer<Idx, Space>
where
    Idx: FungeIndex + Add<Output = Idx>,
    Space: FungeSpace<Idx>,
    Space::Output: From<i32> + Integer,
{
    pub location: Idx,
    pub delta: Idx,
    pub storage_offset: Idx,
    pub stack_stack: LinkedList<Vec<Space::Output>>,
    pub instructions: InstructionSet<Idx, Space>,
}

#[derive(Clone)]
pub struct InstructionSet<Idx, Space>
where
    Idx: FungeIndex + Add<Output = Idx>,
    Space: FungeSpace<Idx>,
    Space::Output: From<i32> + Integer,
{
    instructions: Vec<Option<Rc<dyn FnMut(&mut InstructionPointer<Idx, Space>, &mut Space)>>>,
}

impl<Idx, Space> Debug for InstructionSet<Idx, Space>
where
    Idx: FungeIndex + Add<Output = Idx>,
    Space: FungeSpace<Idx>,
    Space::Output: From<i32> + Integer,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<InstructionSet>");
        Ok(())
    }
}

impl<Space> InstructionPointer<i64, Space>
where
    Space: FungeSpace<i64>,
    Space::Output: From<i32> + Integer,
{
    fn new() -> Self {
        let mut instance = Self {
            location: 0,
            delta: 1,
            storage_offset: 0,
            stack_stack: LinkedList::new(),
            instructions: InstructionSet{ instructions: Vec::new() }
        };
        instance.stack_stack.push_back(Vec::new());
        return instance;
    }
}

impl<Space> InstructionPointer<BefungeVec64, Space>
where
    Space: FungeSpace<BefungeVec64>,
    Space::Output: From<i32> + Integer,
{
    fn new() -> Self {
        let mut instance = Self {
            location: bfvec(0, 0),
            delta: bfvec(1, 0),
            storage_offset: bfvec(0, 0),
            stack_stack: LinkedList::new(),
            instructions: InstructionSet{ instructions: Vec::new() }
        };
        instance.stack_stack.push_back(Vec::new());
        return instance;
    }
}

impl<Idx, Space> InstructionPointer<Idx, Space>
where
    Idx: FungeIndex + Add<Output = Idx>,
    Space: FungeSpace<Idx>,
    Space::Output: From<i32> + Integer,
{
    fn stack(&self) -> &Vec<Space::Output> {
        self.stack_stack.back().unwrap()
    }

    fn stack_mut(&mut self) -> &mut Vec<Space::Output> {
        self.stack_stack.back_mut().unwrap()
    }

    fn pop(&mut self) -> Space::Output {
        match self.stack_mut().pop() {
            Some(v) => v,
            None => Space::Output::from(0 as i32),
        }
    }

    fn push(&mut self, v: Space::Output) {
        self.stack_mut().push(v)
    }
}

#[cfg(test)]
mod tests {
    use super::super::PagedFungeSpace;
    use super::*;

    #[test]
    fn test_stack() {
        let mut ip = InstructionPointer::<BefungeVec64, PagedFungeSpace<BefungeVec64, i64>>::new();

        assert_eq!(ip.pop(), 0);
        ip.push(1);
        ip.push(2);
        assert_eq!(ip.pop(), 2);
        ip.push(3);
        assert_eq!(ip.pop(), 3);
        assert_eq!(ip.pop(), 1);
        ip.push(4);
        ip.push(5);

        ip.stack_stack.push_back(Vec::new());
        assert_eq!(ip.pop(), 0);

        ip.stack_stack.pop_back();
        assert_eq!(ip.pop(), 5);
        assert_eq!(ip.stack().len(), 1);
    }
}
