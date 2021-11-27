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

#![cfg(all(feature = "ncurses", not(target_family = "wasm")))]

use std::cell::RefCell;

use ncurses as nc;
use ncurses::constants::ERR;

use hashbrown::HashMap;
use num::ToPrimitive;

use crate::interpreter::{
    instruction_set::{sync_instruction, Instruction},
    Funge, InstructionPointer, InstructionResult,
};

thread_local! {
    static STDSCR: RefCell<Option<nc::WINDOW>> = RefCell::default();
}

/// From https://web.archive.org/web/20070525220700/http://www.jess2.net:80/code/funge/myexts.txt
///
/// "NCRS" 0x4E435253
/// B ( -- )        Beep or visible beep.
/// E (m -- )       Set echo mode to m (1 == echo, 0 == noecho).
/// G ( -- c)       get character c, modified by various flags
/// I (m -- )       initialize curses mode if m == 1, else end curses mode.
/// K (m -- )       set keypad mode to m (1 == keypad, 0 == nokeypad)
/// M (x y -- )     move cursor to x,y
/// N (m -- )       toggle input mode to m (1 == wait for newline, 0 == cbreak)
/// R ( -- )        refresh(update) window.
/// U (c -- )       unget character c. only guaranteed to work once.
/// P (c -- )       put the character c at the current cursor location.
/// S (0gnirts -- ) write given string at current cursor location.
/// C (m -- )       clear all or part of the screen. m can be one of 0(whole
///                 screen), 1(end of line), or 2(bottom of screen).
///
/// All functions act as r on error.  K is useful for getting KEY_foo codes,
/// i.e. arrow keys, other special keys.  R must be called for the results of
/// other operations to be displayed. You *must* call 'I' at the beginning
/// *and* end of each program that uses NCRS.
///
pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('B', sync_instruction(beep));
    layer.insert('E', sync_instruction(echo_mode));
    layer.insert('G', sync_instruction(getch));
    layer.insert('I', sync_instruction(init_curses));
    layer.insert('K', sync_instruction(keypad_mode));
    layer.insert('M', sync_instruction(move_cursor));
    layer.insert('N', sync_instruction(input_mode));
    layer.insert('R', sync_instruction(refresh));
    layer.insert('U', sync_instruction(ungetch));
    layer.insert('P', sync_instruction(addch));
    layer.insert('S', sync_instruction(addstr));
    layer.insert('C', sync_instruction(clear));

    ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> bool {
    ip.instructions
        .pop_layer(&['B', 'E', 'G', 'I', 'K', 'M', 'N', 'R', 'U', 'P', 'P', 'C'])
}

fn beep<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if nc::flash() == ERR {
        ip.reflect()
    }
    InstructionResult::Continue
}

fn echo_mode<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let m = ip.pop().to_i32().unwrap_or(-1);
    if match m {
        0 => nc::noecho(),
        1 => nc::echo(),
        _ => ERR,
    } == ERR
    {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn getch<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let c = nc::getch();
    if c == ERR {
        ip.reflect();
    } else {
        ip.push(c.into());
    }
    InstructionResult::Continue
}

fn init_curses<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    STDSCR.with(|stdscr_rc| {
        let m = ip.pop().to_i32().unwrap_or_default();
        if m == 1 {
            stdscr_rc.replace(Some(nc::initscr()));
        } else {
            stdscr_rc.borrow_mut().take();
            if nc::endwin() == ERR {
                ip.reflect();
            }
        }
        InstructionResult::Continue
    })
}

fn keypad_mode<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    STDSCR.with(|stdscr_rc| {
        if let Some(stdscr) = *(stdscr_rc.borrow()) {
            let m = ip.pop().to_i32().unwrap_or(-1);
            if match m {
                0 => nc::keypad(stdscr, false),
                1 => nc::keypad(stdscr, true),
                _ => ERR,
            } == ERR
            {
                ip.reflect();
            }
        } else {
            ip.reflect();
        }
        InstructionResult::Continue
    })
}

fn move_cursor<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let y = ip.pop().to_i32().unwrap_or_default();
    let x = ip.pop().to_i32().unwrap_or_default();
    if nc::mv(x, y) == ERR {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn input_mode<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let m = ip.pop().to_i32().unwrap_or(-1);
    if match m {
        0 => nc::cbreak(),
        1 => nc::nocbreak(),
        _ => ERR,
    } == ERR
    {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn refresh<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if nc::refresh() == ERR {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn ungetch<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let c = ip.pop().to_i32().unwrap_or_default();
    if nc::ungetch(c) == ERR {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn addch<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let c = ip.pop().to_u32().unwrap_or_default() as nc::chtype;
    if nc::addch(c) == ERR {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn addstr<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let s = ip.pop_0gnirts();
    if nc::addstr(&s) == ERR {
        ip.reflect();
    }
    InstructionResult::Continue
}

fn clear<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let m = ip.pop().to_i32().unwrap_or(-1);
    if match m {
        0 => nc::clear(),
        1 => nc::clrtoeol(),
        2 => nc::clrtobot(),
        _ => ERR,
    } == ERR
    {
        ip.reflect();
    }
    InstructionResult::Continue
}
