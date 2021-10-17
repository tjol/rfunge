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
    bfvec, new_befunge_interpreter, read_funge_src, safe_fingerprints, BefungeVec, ExecMode,
    FungeSpace, IOMode, Interpreter, InterpreterEnv, PagedFungeSpace, ProgramResult, RunMode,
};

// --------------------------------------------------------
// WASM API
// --------------------------------------------------------

use wasm_bindgen::prelude::{wasm_bindgen, JsValue};
use wasm_bindgen::JsCast;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use std::cmp::min;
use std::io;
use std::io::{Read, Write};

#[wasm_bindgen]
extern "C" {
    pub type JSEnvInterface;

    #[wasm_bindgen(method, js_name = "writeOutput")]
    fn write_output(this: &JSEnvInterface, s: &str);

    #[wasm_bindgen(method, js_name = "warn")]
    fn warn(this: &JSEnvInterface, msg: &str);

    #[wasm_bindgen(method, getter, js_name = "envVars")]
    fn env_vars(this: &JSEnvInterface) -> js_sys::Object;
}

pub struct JSEnv {
    inner: JSEnvInterface,
}

impl Write for JSEnv {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Ok(s) = std::str::from_utf8(buf) {
            self.inner.write_output(s);
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
    fn handprint(&self) -> i32 {
        // alternative handprint for the WASM version
        0x52464e57 // RFNW
    }
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
        self.inner.warn(msg);
    }

    fn is_io_buffered(&self) -> bool {
        false
    }

    fn is_fingerprint_enabled(&self, fpr: i32) -> bool {
        safe_fingerprints().into_iter().any(|f| f == fpr)
    }

    fn env_vars(&mut self) -> Vec<(String, String)> {
        let js_env_vars = self.inner.env_vars();
        let entries: js_sys::Array = js_sys::Object::entries(&js_env_vars);
        entries
            .iter()
            .filter_map(|e| e.dyn_into::<js_sys::Array>().ok())
            .filter_map(|e| Some((e.get(0).as_string()?, e.get(1).as_string()?)))
            .collect()
    }

    fn have_execute(&self) -> ExecMode {
        ExecMode::SameShell
    }

    fn execute_command(&mut self, command: &str) -> i32 {
        match js_sys::eval(command) {
            Ok(val) => {
                if val.is_null() || val.is_undefined() {
                    0
                } else if let Some(n) = val.as_f64() {
                    n as i32
                } else if val.is_truthy() {
                    0
                } else {
                    1
                }
            }
            Err(_) => 1,
        }
    }
}

type WebBefungeInterp = Interpreter<BefungeVec<i32>, PagedFungeSpace<BefungeVec<i32>, i32>, JSEnv>;

#[wasm_bindgen]
pub struct BefungeInterpreter {
    interpreter: WebBefungeInterp,
}

#[wasm_bindgen]
impl BefungeInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new(env: JSEnvInterface) -> Self {
        // console_error_panic_hook::set_once();
        let real_env = JSEnv { inner: env };
        Self {
            interpreter: new_befunge_interpreter::<i32, _>(real_env),
        }
    }

    pub fn close(self) -> JSEnvInterface {
        self.interpreter.env.inner
    }

    #[wasm_bindgen(js_name = "loadSrc")]
    pub fn load_src(&mut self, src: &str) {
        read_funge_src(&mut self.interpreter.space, src);
    }

    #[wasm_bindgen(js_name = "replaceSrc")]
    pub fn replace_src(&mut self, src: &str) {
        self.interpreter.space = PagedFungeSpace::new_with_page_size(bfvec(80, 25));
        read_funge_src(&mut self.interpreter.space, src);
    }

    pub fn run(&mut self) -> i32 {
        match self.interpreter.run(RunMode::Run) {
            ProgramResult::Done(returncode) => returncode,
            _ => -1,
        }
    }

    pub fn run_limited(&mut self, loop_limit: u32) -> Option<i32> {
        for _ in 0..loop_limit {
            match self.interpreter.run(RunMode::Step) {
                ProgramResult::Done(returncode) => {
                    return Some(returncode);
                }
                ProgramResult::Panic => return Some(-1),
                ProgramResult::Paused => {}
            }
        }
        None
    }

    pub fn step(&mut self) -> Option<i32> {
        match self.interpreter.run(RunMode::Step) {
            ProgramResult::Done(returncode) => Some(returncode),
            ProgramResult::Panic => Some(-1),
            ProgramResult::Paused => None,
        }
    }

    #[wasm_bindgen(js_name = "ipCount")]
    pub fn ip_count(&self) -> usize {
        self.interpreter.ips.len()
    }

    #[wasm_bindgen(js_name = "ipLocation")]
    pub fn ip_location(&self, ip_idx: usize) -> Option<Vec<i32>> {
        let loc = self.interpreter.ips.get(ip_idx)?.location;
        Some(vec![loc.x, loc.y])
    }

    #[wasm_bindgen(js_name = "stackCount")]
    pub fn stack_count(&self, ip_idx: usize) -> usize {
        self.interpreter
            .ips
            .get(ip_idx)
            .map(|ip| ip.stack_stack.len())
            .unwrap_or(0)
    }

    #[wasm_bindgen(js_name = "getStack")]
    pub fn get_stack(&self, ip_idx: usize, stack_idx: usize) -> Option<Vec<i32>> {
        self.interpreter
            .ips
            .get(ip_idx)
            .and_then(|ip| ip.stack_stack.get(stack_idx))
            .map(|v| v.clone())
    }

    #[wasm_bindgen(js_name = "getSrc")]
    pub fn get_src(&self) -> String {
        let mut start = self.interpreter.space.min_idx().unwrap_or(bfvec(0, 0));
        start = bfvec(min(0, start.x), min(0, start.y));
        let end_incl = self.interpreter.space.max_idx().unwrap_or(bfvec(0, 0));
        let size = bfvec(end_incl.x - start.x + 1, end_incl.y - start.y + 1);
        SrcIO::get_src_str(&self.interpreter.space, &start, &size, true)
    }

    #[wasm_bindgen(js_name = "getSrcLines")]
    pub fn get_src_lines(&self) -> Vec<JsValue> {
        let mut start = self.interpreter.space.min_idx().unwrap_or(bfvec(0, 0));
        start = bfvec(min(0, start.x), min(0, start.y));
        let end_incl = self.interpreter.space.max_idx().unwrap_or(bfvec(0, 0));
        let line_len = end_incl.x - start.x + 1;

        (start.y..(end_incl.y + 1))
            .map(|y| {
                SrcIO::get_src_str(
                    &self.interpreter.space,
                    &bfvec(start.x, y),
                    &bfvec(line_len, 1),
                    true,
                )
            })
            .map(|s| JsValue::from_str(&s))
            .collect()
    }
}
