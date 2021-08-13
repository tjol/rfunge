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

#![cfg(target_arch = "wasm32")]

use crate::{
    new_befunge_interpreter, read_befunge, BefungeVec, IOMode, Interpreter,
    InterpreterEnv, PagedFungeSpace,
};

// --------------------------------------------------------
// WASM API
// --------------------------------------------------------

use wasm_bindgen::prelude::wasm_bindgen;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use std::io;
use std::io::{Read, Write};

// It is possible to call JavaScript closures from WASM, but just using
// standard global functions is easier as this is only an internal
// interface
#[wasm_bindgen(raw_module = "../wasm_io.js")]
extern "C" {
    fn write_rfunge_output(s: &str);
    fn write_rfunge_warning(msg: &str);
}

pub struct JSEnv {}

impl Write for JSEnv {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Ok(s) = std::str::from_utf8(buf) {
            write_rfunge_output(s);
            Ok(s.len())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "UTF-8 error"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for JSEnv {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        // EOF by default [TODO]
        Ok(0)
    }
}

impl InterpreterEnv for JSEnv {
    fn get_iomode(&self) -> IOMode {
        IOMode::Text
    }
    fn output_writer(&mut self) -> &mut dyn Write {
        self
    }

    fn input_reader(&mut self) -> &mut dyn Read {
        self
    }

    fn warn(&mut self, msg: &str) {
        write_rfunge_warning(msg);
    }
}

type WebBefungeInterp = Interpreter<BefungeVec<i32>, PagedFungeSpace<BefungeVec<i32>, i32>, JSEnv>;

#[wasm_bindgen]
pub fn new_interpreter() -> *mut WebBefungeInterp {
    let interp = Box::new(new_befunge_interpreter::<i32, _>(JSEnv {}));
    // console_error_panic_hook::set_once();
    return Box::into_raw(interp);
}

#[wasm_bindgen]
pub fn free_interpreter(interp: *mut WebBefungeInterp) {
    unsafe {
        Box::from_raw(interp);
    }
}

#[wasm_bindgen]
pub fn load_src(interp: *mut WebBefungeInterp, src: &str) {
    let interp_ref = unsafe { &mut (*interp) };
    read_befunge(&mut interp_ref.space, src);
}

#[wasm_bindgen]
pub fn run(interp: *mut WebBefungeInterp) {
    let interp_ref = unsafe { &mut (*interp) };
    interp_ref.run();
}
