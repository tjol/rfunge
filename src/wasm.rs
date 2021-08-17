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

use crate::fungespace::SrcIO;
use crate::{
    bfvec, new_befunge_interpreter, read_funge_src, safe_fingerprints, BefungeVec, FungeSpace,
    IOMode, Interpreter, InterpreterEnv, PagedFungeSpace, ProgramResult, RunMode,
};

// --------------------------------------------------------
// WASM API
// --------------------------------------------------------

use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use std::cmp::min;
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

    fn is_io_buffered(&self) -> bool {
        false
    }

    fn is_fingerprint_enabled(&self, fpr: i32) -> bool {
        safe_fingerprints().into_iter().any(|f| f == fpr)
    }
}

type WebBefungeInterp = Interpreter<BefungeVec<i32>, PagedFungeSpace<BefungeVec<i32>, i32>, JSEnv>;

#[wasm_bindgen]
pub struct BefungeInterpreter {
    interpreter: Option<WebBefungeInterp>,
}

#[wasm_bindgen]
impl BefungeInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // console_error_panic_hook::set_once();
        Self { interpreter: None }
    }

    pub fn init(&mut self) {
        self.interpreter = Some(new_befunge_interpreter::<i32, _>(JSEnv {}));
    }

    pub fn close(&mut self) {
        self.interpreter = None;
    }

    #[wasm_bindgen(js_name = "loadSrc")]
    pub fn load_src(&mut self, src: &str) {
        match &mut self.interpreter {
            None => {
                self.init();
                self.load_src(src);
            }
            Some(interpreter) => {
                read_funge_src(&mut interpreter.space, src);
            }
        }
    }

    #[wasm_bindgen(js_name = "replaceSrc")]
    pub fn replace_src(&mut self, src: &str) {
        match &mut self.interpreter {
            None => {
                self.init();
                self.load_src(src);
            }
            Some(interpreter) => {
                interpreter.space = PagedFungeSpace::new_with_page_size(bfvec(80, 25));
                read_funge_src(&mut interpreter.space, src);
            }
        }
    }

    pub fn run(&mut self) -> i32 {
        match &mut self.interpreter {
            None => -1,
            Some(interpreter) => match interpreter.run(RunMode::Run) {
                ProgramResult::Done(returncode) => returncode,
                _ => -1,
            },
        }
    }

    pub fn run_limited(&mut self, loop_limit: u32) -> Option<i32> {
        match &mut self.interpreter {
            None => Some(-1),
            Some(interpreter) => {
                for _ in 0..loop_limit {
                    match interpreter.run(RunMode::Step) {
                        ProgramResult::Done(returncode) => {
                            return Some(returncode);
                        }
                        ProgramResult::Panic => return Some(-1),
                        ProgramResult::Paused => {}
                    }
                }
                None
            }
        }
    }

    pub fn step(&mut self) -> Option<i32> {
        match &mut self.interpreter {
            None => Some(-1),
            Some(interpreter) => match interpreter.run(RunMode::Step) {
                ProgramResult::Done(returncode) => Some(returncode),
                ProgramResult::Panic => Some(-1),
                ProgramResult::Paused => None,
            },
        }
    }

    #[wasm_bindgen(js_name = "ipCount")]
    pub fn ip_count(&self) -> usize {
        self.interpreter.as_ref().map(|i| i.ips.len()).unwrap_or(0)
    }

    #[wasm_bindgen(js_name = "ipLocation")]
    pub fn ip_location(&self, ip_idx: usize) -> Option<Vec<i32>> {
        self.interpreter
            .as_ref()
            .and_then(|i| Some(i.ips.get(ip_idx)?.location))
            .and_then(|loc| Some(vec![loc.x, loc.y]))
    }

    #[wasm_bindgen(js_name = "stackCount")]
    pub fn stack_count(&self, ip_idx: usize) -> usize {
        self.interpreter
            .as_ref()
            .and_then(|i| Some(i.ips.get(ip_idx)?.stack_stack.len()))
            .unwrap_or(0)
    }

    #[wasm_bindgen(js_name = "getStack")]
    pub fn get_stack(&self, ip_idx: usize, stack_idx: usize) -> Option<Vec<i32>> {
        self.interpreter
            .as_ref()
            .and_then(|i| i.ips.get(ip_idx)?.stack_stack.get(stack_idx))
            .map(|v| v.clone())
    }

    #[wasm_bindgen(js_name = "getSrc")]
    pub fn get_src(&self) -> String {
        self.interpreter
            .as_ref()
            .map(|i| {
                let mut start = i.space.min_idx().unwrap_or(bfvec(0, 0));
                start = bfvec(min(0, start.x), min(0, start.y));
                let end_incl = i.space.max_idx().unwrap_or(bfvec(0, 0));
                let size = bfvec(end_incl.x - start.x + 1, end_incl.y - start.y + 1);
                SrcIO::get_src_str(&i.space, &start, &size, true)
            })
            .unwrap_or(String::new())
    }

    #[wasm_bindgen(js_name = "getSrcLines")]
    pub fn get_src_lines(&self) -> Vec<JsValue> {
        self.interpreter
            .as_ref()
            .map(|i| {
                let mut start = i.space.min_idx().unwrap_or(bfvec(0, 0));
                start = bfvec(min(0, start.x), min(0, start.y));
                let end_incl = i.space.max_idx().unwrap_or(bfvec(0, 0));
                let line_len = end_incl.x - start.x + 1;

                (start.y..(end_incl.y + 1))
                    .map(|y| {
                        SrcIO::get_src_str(&i.space, &bfvec(start.x, y), &bfvec(line_len, 1), true)
                    })
                    .map(|s| JsValue::from_str(&s))
                    .collect()
            })
            .unwrap_or(Vec::new())
    }
}
