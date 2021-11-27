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

#[cfg(target_family = "wasm")]
use serde::{Deserialize, Serialize};

use super::string_to_fingerprint;
use crate::interpreter::instruction_set::{sync_instruction, Instruction, InstructionResult};
use crate::interpreter::{Funge, InstructionPointer, InterpreterEnv};

#[cfg_attr(target_family = "wasm", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[cfg_attr(target_family = "wasm", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[cfg_attr(target_family = "wasm", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct Line {
    pub from: Point,
    pub to: Point,
    pub colour: Colour,
}

#[cfg_attr(target_family = "wasm", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct Dot {
    pub pos: Point,
    pub colour: Colour,
}

/// Trait for a general turtle robot implementation
///
/// This could be anything from an HTML5 canvas to a LEGO Mindstorms robot
pub trait TurtleRobot {
    fn turn_left(&mut self, degrees: i32);
    fn set_heading(&mut self, degrees: i32);
    fn heading(&self) -> i32;
    fn set_pen(&mut self, down: bool);
    fn is_pen_down(&self) -> bool;
    fn forward(&mut self, pixels: i32);
    fn set_colour(&mut self, rgb: Colour);
    fn clear_with_colour(&mut self, rgb: Colour);
    fn display(&mut self, show: bool);
    fn teleport(&mut self, dest: Point);
    fn position(&self) -> Point;
    fn bounds(&self) -> (Point, Point);
    fn print(&mut self);
}

/// Trait for a typical graphical display (could also be a bitmap of vector graphic)
/// used by the virtual turtle
pub trait TurtleDisplay {
    fn display(&mut self, show: bool);
    fn display_visible(&self) -> bool;
    fn draw(&mut self, background: Option<Colour>, lines: &[Line], dots: &[Dot]);
    fn print(&mut self, background: Option<Colour>, lines: &[Line], dots: &[Dot]);
}

/// Struct implementing TurtleRobot for a generic graphical output
pub struct SimpleRobot<D: TurtleDisplay> {
    display: D,
    lines: Vec<Line>,
    dots: Vec<Dot>,
    heading: i32,
    position: Point,
    pen_down: bool,
    colour: Colour,
    background: Option<Colour>,
    have_drawn: bool,
}

/// Type expected from env.fingerprint_support_library()
pub type TurtleRobotBox = Box<dyn TurtleRobot>;

impl<D: TurtleDisplay> SimpleRobot<D> {
    pub fn new(display: D) -> Self {
        Self {
            display,
            lines: vec![],
            dots: vec![],
            heading: 0,
            position: Point { x: 0, y: 0 },
            pen_down: false,
            colour: Colour { r: 0, g: 0, b: 0 },
            background: None,
            have_drawn: false,
        }
    }

    fn redraw(&mut self, print: bool) {
        if print || self.display.display_visible() {
            let mut all_dots;
            let mut dots: &[Dot] = &self.dots;
            if self.pen_down && !self.have_drawn {
                all_dots = Some(self.dots.clone());
                all_dots.as_mut().unwrap().push(Dot {
                    pos: self.position,
                    colour: self.colour,
                });
                dots = all_dots.as_ref().unwrap();
            }
            if print {
                self.display.print(self.background, &self.lines, dots);
            } else {
                self.display.draw(self.background, &self.lines, dots);
            }
        }
    }
}

impl<D: TurtleDisplay + 'static> SimpleRobot<D> {
    pub fn new_in_box(display: D) -> TurtleRobotBox {
        Box::new(Self::new(display))
    }
}

pub fn calc_bounds<'a, LI, DI>(lines: LI, dots: DI) -> (Point, Point)
where
    LI: Iterator<Item = &'a Line>,
    DI: Iterator<Item = &'a Dot>,
{
    let points = lines
        .flat_map(|l| [l.from, l.to])
        .chain(dots.map(|d| d.pos));
    calc_bounds_from_points(points)
}

pub fn calc_bounds_from_points<I>(points: I) -> (Point, Point)
where
    I: Iterator<Item = Point>,
{
    let mut any = false;
    let mut min_x = 0;
    let mut max_x = 0;
    let mut min_y = 0;
    let mut max_y = 0;
    for p in points {
        min_x = if any { std::cmp::min(min_x, p.x) } else { p.x };
        max_x = if any { std::cmp::max(max_x, p.x) } else { p.x };
        min_y = if any { std::cmp::min(min_y, p.y) } else { p.y };
        max_y = if any { std::cmp::max(max_y, p.y) } else { p.y };
        any = true;
    }
    if any {
        (Point { x: min_x, y: min_y }, Point { x: max_x, y: max_y })
    } else {
        (Point { x: 0, y: 0 }, Point { x: 0, y: 0 })
    }
}

impl<D: TurtleDisplay> TurtleRobot for SimpleRobot<D> {
    fn turn_left(&mut self, degrees: i32) {
        self.heading -= degrees;
    }
    fn set_heading(&mut self, degrees: i32) {
        self.heading = degrees;
    }
    fn heading(&self) -> i32 {
        self.heading
    }
    fn set_pen(&mut self, down: bool) {
        if self.pen_down && !down && !self.have_drawn {
            // make a dot
            self.dots.push(Dot {
                pos: self.position,
                colour: self.colour,
            });
        } else if !self.pen_down {
            self.have_drawn = false;
        }
        self.pen_down = down;
        self.redraw(false);
    }
    fn is_pen_down(&self) -> bool {
        self.pen_down
    }
    fn forward(&mut self, pixels: i32) {
        let heading_rad = (self.heading as f64) / 180.0 * std::f64::consts::PI;
        let dest = Point {
            x: self.position.x + (pixels as f64 * heading_rad.cos()).round() as i32,
            y: self.position.y + (pixels as f64 * heading_rad.sin()).round() as i32,
        };
        if self.pen_down {
            self.lines.push(Line {
                from: self.position,
                to: dest,
                colour: self.colour,
            });
            self.have_drawn = true;
            self.redraw(false)
        }
        self.position = dest;
    }
    fn set_colour(&mut self, rgb: Colour) {
        self.colour = rgb;
    }
    fn clear_with_colour(&mut self, rgb: Colour) {
        self.background = Some(rgb);
        self.lines.clear();
        self.dots.clear();
        self.have_drawn = false;
        self.redraw(false)
    }
    fn display(&mut self, show: bool) {
        self.display.display(show);
        self.redraw(false);
    }
    fn teleport(&mut self, dest: Point) {
        if self.pen_down && !self.have_drawn {
            // Leave a dot at the old location
            self.dots.push(Dot {
                pos: self.position,
                colour: self.colour,
            });
        }
        self.position = dest;
        self.redraw(false);
    }
    fn position(&self) -> Point {
        self.position
    }
    fn bounds(&self) -> (Point, Point) {
        calc_bounds(self.lines.iter(), self.dots.iter())
    }
    fn print(&mut self) {
        self.redraw(true);
    }
}

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
pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> bool {
    // Do we have TURT support from the environment?
    if env
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
        ip.instructions.add_layer(layer);
        true
    }
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    ip.instructions
        .pop_layer(&"LRHFBPCNDTEAQUI".chars().collect::<Vec<char>>())
}

fn pop_colour<F: Funge>(ip: &mut InstructionPointer<F>) -> Colour {
    let colour_24bit = ip.pop().to_i32().unwrap_or_default();
    Colour {
        r: ((colour_24bit & 0xff0000) >> 16) as u8,
        g: ((colour_24bit & 0xff00) >> 8) as u8,
        b: (colour_24bit & 0xff) as u8,
    }
}

fn trun_left<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let angle = ip.pop().to_i32().unwrap_or_default();
        robot.turn_left(angle);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn turn_right<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let angle = ip.pop().to_i32().unwrap_or_default();
        robot.turn_left(-angle);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn set_heading<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let angle = ip.pop().to_i32().unwrap_or_default();
        robot.set_heading(angle);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn forward<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let dist = ip.pop().to_i32().unwrap_or_default();
        robot.forward(dist);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn back<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let dist = ip.pop().to_i32().unwrap_or_default();
        robot.forward(-dist);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn pen_position<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let pos = ip.pop() == 1.into();
        robot.set_pen(pos);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn pen_colour<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        robot.set_colour(pop_colour(ip));
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn clear_paper<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        robot.clear_with_colour(pop_colour(ip));
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn display<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let disp = ip.pop() == 1.into();
        robot.display(disp);
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn teleport<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        let y = ip.pop().to_i32().unwrap_or_default();
        let x = ip.pop().to_i32().unwrap_or_default();
        robot.teleport(Point { x, y });
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn query_pen<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_ref::<TurtleRobotBox>())
    {
        ip.push(if robot.is_pen_down() {
            1.into()
        } else {
            0.into()
        });
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn query_heading<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_ref::<TurtleRobotBox>())
    {
        ip.push(robot.heading().into());
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn query_position<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_ref::<TurtleRobotBox>())
    {
        let Point { x, y } = robot.position();
        ip.push(x.into());
        ip.push(y.into());
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn query_bounds<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_ref::<TurtleRobotBox>())
    {
        let (
            Point { x: left, y: top },
            Point {
                x: right,
                y: bottom,
            },
        ) = robot.bounds();
        ip.push(left.into());
        ip.push(top.into());
        ip.push(right.into());
        ip.push(bottom.into());
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn print_drawing<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if let Some(robot) = env
        .fingerprint_support_library(string_to_fingerprint("TURT"))
        .and_then(|lib| lib.downcast_mut::<TurtleRobotBox>())
    {
        robot.print();
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}
