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

use crate::interpreter::instruction_set::{
    sync_instruction, Instruction, InstructionContext, InstructionResult, InstructionSet,
};
use crate::interpreter::Funge;

/// From the catseye library
///
/// Fingerprint 0x524f4d41 ('ROMA')
///
/// After successfully loading ROMA, the instructions `C`, `D`, `I`, `L`,
/// `M`, `V`, and `X` take on new semantics.
///
/// -   `C` pushes 100 onto the stack.
/// -   `D` pushes 500 onto the stack.
/// -   `I` pushes 1 onto the stack.
/// -   `L` pushes 50 onto the stack.
/// -   `M` pushes 1000 onto the stack.
/// -   `V` pushes 5 onto the stack.
/// -   `X` pushes 10 onto the stack.
///
/// Note that these are just digits, you still have to do the arithmetic
/// yourself. Executing `MCMLXXXIV` will not leave 1984 on the stack. But
/// executing `MCM\-+LXXX+++IV\-++` should.
pub fn load<F: Funge>(instructionset: &mut InstructionSet<F>) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('I', sync_instruction(unum));
    layer.insert('V', sync_instruction(quinque));
    layer.insert('X', sync_instruction(decem));
    layer.insert('L', sync_instruction(quinquaginta));
    layer.insert('C', sync_instruction(centum));
    layer.insert('D', sync_instruction(quingenti));
    layer.insert('M', sync_instruction(mille));

    instructionset.add_layer(layer);
    true
}

pub fn unload<F: Funge>(instructionset: &mut InstructionSet<F>) -> bool {
    instructionset.pop_layer(&['I', 'V', 'X', 'L', 'C', 'D', 'M'])
}

fn unum<F: Funge>(mut ctx: InstructionContext<F>) -> (InstructionContext<F>, InstructionResult)
{
    ctx.ip.push(1.into());
    (ctx, InstructionResult::Continue)
}

fn quinque<F: Funge>(mut ctx: InstructionContext<F>) -> (InstructionContext<F>, InstructionResult)
{
    ctx.ip.push(5.into());
    (ctx, InstructionResult::Continue)
}

fn decem<F: Funge>(mut ctx: InstructionContext<F>) -> (InstructionContext<F>, InstructionResult)
{
    ctx.ip.push(10.into());
    (ctx, InstructionResult::Continue)
}

fn quinquaginta<F: Funge>(mut ctx: InstructionContext<F>) -> (InstructionContext<F>, InstructionResult)
{
    ctx.ip.push(50.into());
    (ctx, InstructionResult::Continue)
}

fn centum<F: Funge>(mut ctx: InstructionContext<F>) -> (InstructionContext<F>, InstructionResult)
{
    ctx.ip.push(100.into());
    (ctx, InstructionResult::Continue)
}

fn quingenti<F: Funge>(mut ctx: InstructionContext<F>) -> (InstructionContext<F>, InstructionResult)
{
    ctx.ip.push(500.into());
    (ctx, InstructionResult::Continue)
}

fn mille<F: Funge>(mut ctx: InstructionContext<F>) -> (InstructionContext<F>, InstructionResult)
{
    ctx.ip.push(1000.into());
    (ctx, InstructionResult::Continue)
}
