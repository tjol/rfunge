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

use num::ToPrimitive;

use super::string_to_fingerprint;
use crate::interpreter::instruction_set::{
    sync_instruction, Instruction, InstructionContext, InstructionResult,
};
use crate::interpreter::{Funge, InstructionPointer, InterpreterEnv};

pub trait TurtleRobot {
    fn turn_left(&mut self, degrees: i32);
    fn set_heading(&mut self, degrees: i32);
    fn heading(&self) -> i32;
    fn set_pen(&mut self, down: bool);
    fn is_pen_down(&self) -> bool;
    fn forward(&mut self, pixels: i32);
    fn set_colour(&mut self, r: u8, g: u8, b: u8);
    fn clear_with_colour(&mut self, r: u8, g: u8, b: u8);
    fn display(&mut self, show: bool);
    fn teleport(&mut self, x: i32, y: i32);
    fn position(&self) -> (i32, i32);
    fn bounds(&self) -> ((i32, i32), (i32, i32));
    fn print(&mut self);
}

pub type TurtleRobotBox = Box<dyn TurtleRobot>;

/// From the catseye library
///
/// ### Fingerprint 0x54555254 ('TURT')
///
/// Under development.
///
/// The TURT fingerprint provides a simple interface to a simple "drawing
/// turtle-robot simulator".
///
/// After successfully loading TURT, several instructions take on new
/// semantics.
///
/// These instructions pop one value off the stack:
///
/// -   `L` 'Turn Left' (angle in degrees)
/// -   `R` 'Turn Right' (angle in degrees)
/// -   `H` 'Set Heading' (angle in degrees, relative to 0deg, east)
/// -   `F` 'Forward' (distance in pixels)
/// -   `B` 'Back' (distance in pixels)
/// -   `P` 'Pen Position' (0 = up, 1 = down)
/// -   `C` 'Pen Colour' (24-bit RGB)
/// -   `N` 'Clear Paper with Colour' (24-bit RGB)
/// -   `D` 'Show Display' (0 = no, 1 = yes)
///
/// These pop two values each:
///
/// -   `T` 'Teleport' (x, y coords relative to origin; 00T = home)
///
/// These push one value each:
///
/// -   `E` 'Query Pen' (0 = up, 1 = down)
/// -   `A` 'Query Heading' (positive angle relative to east)
///
/// These push two values each:
///
/// -   `Q` 'Query Position' (x, y coordinates)
///
/// These push four values each:
///
/// -   `U` 'Query Bounds' (two pairs of x, y coordinates)
///
/// And these don't even use the stack:
///
/// -   `I` 'Print current Drawing' (if possible)
///
/// To keep this fingerprint tame, a single Turtle and display is defined to
/// be shared amongst all IP's. The turtle is not defined to wrap if it goes
/// out of bounds (after all this interface might just as well be used to
/// drive a **real** turtle robot.)
pub fn load<F: Funge>(ctx: &mut InstructionContext<F>) -> bool {
    // Do we have TURT support from the environment?
    if ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_ref::<TurtleRobotBox>())
        .is_none()
    {
        false
    } else {
        let mut layer = HashMap::<char, Instruction<F>>::new();
        layer.insert('L', sync_instruction(trun_left));
        layer.insert('R', sync_instruction(turn_right));
        layer.insert('H', sync_instruction(set_heading));
        layer.insert('F', sync_instruction(forward));
        layer.insert('B', sync_instruction(back));
        layer.insert('P', sync_instruction(pen_position));
        layer.insert('C', sync_instruction(pen_colour));
        layer.insert('N', sync_instruction(clear_paper));
        layer.insert('D', sync_instruction(display));
        layer.insert('T', sync_instruction(teleport));
        layer.insert('E', sync_instruction(query_pen));
        layer.insert('A', sync_instruction(query_heading));
        layer.insert('Q', sync_instruction(query_position));
        layer.insert('U', sync_instruction(query_bounds));
        layer.insert('I', sync_instruction(print_drawing));
        ctx.ip.instructions.add_layer(layer);
        true
    }
}

pub fn unload<F: Funge>(ctx: &mut InstructionContext<F>) -> bool {
    ctx.ip
        .instructions
        .pop_layer(&"LRHFBPCNDTEAQUI".chars().collect::<Vec<char>>())
}

fn pop_colour<F: Funge>(ip: &mut InstructionPointer<F>) -> (u8, u8, u8) {
    let colour_24bit = ip.pop().to_i32().unwrap_or_default();
    let r = ((colour_24bit & 0xff0000) >> 16) as u8;
    let g = ((colour_24bit & 0xff00) >> 8) as u8;
    let b = (colour_24bit & 0xff) as u8;
    return (r, g, b);
}

fn trun_left<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let angle = ctx.ip.pop().to_i32().unwrap_or_default();
        robot.turn_left(angle);
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn turn_right<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let angle = ctx.ip.pop().to_i32().unwrap_or_default();
        robot.turn_left(-angle);
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn set_heading<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let angle = ctx.ip.pop().to_i32().unwrap_or_default();
        robot.set_heading(angle);
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn forward<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let dist = ctx.ip.pop().to_i32().unwrap_or_default();
        robot.forward(dist);
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn back<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let dist = ctx.ip.pop().to_i32().unwrap_or_default();
        robot.forward(-dist);
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn pen_position<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let pos = ctx.ip.pop() == 1.into();
        robot.set_pen(pos);
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn pen_colour<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let (r, g, b) = pop_colour(&mut ctx.ip);
        robot.set_colour(r, g, b);
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn clear_paper<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let (r, g, b) = pop_colour(&mut ctx.ip);
        robot.clear_with_colour(r, g, b);
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn display<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let disp = ctx.ip.pop() == 1.into();
        robot.display(disp);
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn teleport<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let y = ctx.ip.pop().to_i32().unwrap_or_default();
        let x = ctx.ip.pop().to_i32().unwrap_or_default();
        robot.teleport(x, y);
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn query_pen<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_ref::<TurtleRobotBox>())
    {
        ctx.ip.push(if robot.is_pen_down() {
            1.into()
        } else {
            0.into()
        });
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn query_heading<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_ref::<TurtleRobotBox>())
    {
        ctx.ip.push(robot.heading().into());
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn query_position<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_ref::<TurtleRobotBox>())
    {
        let (x, y) = robot.position();
        ctx.ip.push(x.into());
        ctx.ip.push(y.into());
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn query_bounds<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_ref::<TurtleRobotBox>())
    {
        let ((left, top), (right, bottom)) = robot.bounds();
        ctx.ip.push(left.into());
        ctx.ip.push(top.into());
        ctx.ip.push(right.into());
        ctx.ip.push(bottom.into());
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

fn print_drawing<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult {
    if let Some(robot) = ctx
        .env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        robot.print();
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}
