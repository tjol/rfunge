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

#![cfg(not(target_family = "wasm"))]

use std::io::stdout;

use crossterm::{
    cursor::{MoveDown, MoveTo, MoveUp},
    execute,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use hashbrown::HashMap;
use num::ToPrimitive;

use crate::interpreter::instruction_set::{
    sync_instruction, Instruction, InstructionContext, InstructionResult,
};
use crate::interpreter::Funge;

/// From the rcFunge docs
///
/// "TERM" 0x5445524D
/// C   ( -- )  Clear the screen
/// D   ( n -- )    Move cursor down n lines
/// G   (x y -- )   Put cursor at position x,y (home is 0,0)
/// H   ( -- )  Move cursor to home
/// L   ( -- )  Clear to end of line
/// S   ( -- )  Clear to end of screen
/// U   ( n -- )    Move cursor up n lines
///
pub fn load<F: Funge>(ctx: &mut InstructionContext<F>) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('C', sync_instruction(clear_screen));
    layer.insert('D', sync_instruction(down));
    layer.insert('G', sync_instruction(goto));
    layer.insert('H', sync_instruction(home));
    layer.insert('L', sync_instruction(clear_to_eol));
    layer.insert('S', sync_instruction(clear_to_eos));
    layer.insert('U', sync_instruction(up));

    ctx.ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(ctx: &mut InstructionContext<F>) -> bool {
    ctx.ip
        .instructions
        .pop_layer(&['C', 'D', 'G', 'H', 'L', 'S', 'U'])
}

fn clear_screen<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    let mut stdout = stdout();
    if stdout.execute(Clear(ClearType::All)).is_err() {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn down<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    (|| -> Option<()> {
        let mut stdout = stdout();
        let n = ctx.ip.pop().to_u16()?;
        execute!(stdout, MoveDown(n)).ok()
    })()
    .unwrap_or_else(|| ctx.ip.reflect());
    InstructionResult::Continue
}

fn goto<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    (|| -> Option<()> {
        let mut stdout = stdout();
        let y = ctx.ip.pop().to_u16()?;
        let x = ctx.ip.pop().to_u16()?;
        execute!(stdout, MoveTo(x, y)).ok()
    })()
    .unwrap_or_else(|| ctx.ip.reflect());
    InstructionResult::Continue
}

fn home<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    let mut stdout = stdout();
    if stdout.execute(MoveTo(0, 0)).is_err() {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn clear_to_eol<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    let mut stdout = stdout();
    if stdout.execute(Clear(ClearType::UntilNewLine)).is_err() {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn clear_to_eos<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    let mut stdout = stdout();
    if stdout.execute(Clear(ClearType::FromCursorDown)).is_err() {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn up<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    (|| -> Option<()> {
        let mut stdout = stdout();
        let n = ctx.ip.pop().to_u16()?;
        execute!(stdout, MoveUp(n)).ok()
    })()
    .unwrap_or_else(|| ctx.ip.reflect());
    InstructionResult::Continue
}
