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

#![cfg(target_family = "wasm")]

use std::any::Any;
use std::cmp::min;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_lite::io as f_io;
use futures_lite::io::{AsyncRead, AsyncWrite};

use wasm_bindgen::prelude::{wasm_bindgen, JsValue};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::fungespace::SrcIO;
use crate::interpreter::fingerprints::string_to_fingerprint;
use crate::interpreter::fingerprints::TURT::{
    Colour, Dot, Line, SimpleRobot, TurtleDisplay, TurtleRobotBox,
};
use crate::{
    bfvec, new_befunge_interpreter, read_funge_src, safe_fingerprints, BefungeVec, ExecMode,
    FungeSpace, IOMode, Interpreter, InterpreterEnv, PagedFungeSpace, ProgramResult, RunMode,
};

#[wasm_bindgen]
extern "C" {
    pub type JSEnvInterface;
    pub type JSTurtleDisplay;

    #[wasm_bindgen(method, js_name = "writeOutput")]
    fn write_output(this: &JSEnvInterface, s: &str);
    #[wasm_bindgen(method, js_name = "warn")]
    fn warn(this: &JSEnvInterface, msg: &str);
    #[wasm_bindgen(method, getter, js_name = "envVars")]
    fn env_vars(this: &JSEnvInterface) -> js_sys::Object;
    #[wasm_bindgen(method, js_name = "readInput")]
    fn read_input(this: &JSEnvInterface) -> js_sys::Promise;
    #[wasm_bindgen(method, getter, js_name = "turtleDisplay")]
    fn turtle_display(this: &JSEnvInterface) -> JSTurtleDisplay;

    #[wasm_bindgen(method)]
    fn display(this: &JSTurtleDisplay, show: bool);
    #[wasm_bindgen(method, js_name = "isDisplayVisible")]
    fn display_visible(this: &JSTurtleDisplay) -> bool;
    #[wasm_bindgen(method)]
    fn draw(this: &JSTurtleDisplay, background: JsValue, lines: Vec<JsValue>, dots: Vec<JsValue>);
    #[wasm_bindgen(method)]
    fn print(this: &JSTurtleDisplay, background: JsValue, lines: Vec<JsValue>, dots: Vec<JsValue>);
}

struct TurtleDisplayWrapper {
    display: JSTurtleDisplay,
}

impl TurtleDisplay for TurtleDisplayWrapper {
    fn display(&mut self, show: bool) {
        self.display.display(show);
    }
    fn display_visible(&self) -> bool {
        self.display.display_visible()
    }
    fn draw(&mut self, background: Option<Colour>, lines: &[Line], dots: &[Dot]) {
        self.display.draw(
            background
                .as_ref()
                .and_then(|c| JsValue::from_serde(c).ok())
                .unwrap_or(JsValue::NULL),
            lines
                .iter()
                .filter_map(|l| JsValue::from_serde(l).ok())
                .collect(),
            dots.iter()
                .filter_map(|d| JsValue::from_serde(d).ok())
                .collect(),
        )
    }
    fn print(&mut self, background: Option<Colour>, lines: &[Line], dots: &[Dot]) {
        self.display.print(
            background
                .as_ref()
                .and_then(|c| JsValue::from_serde(c).ok())
                .unwrap_or(JsValue::NULL),
            lines
                .iter()
                .filter_map(|l| JsValue::from_serde(l).ok())
                .collect(),
            dots.iter()
                .filter_map(|d| JsValue::from_serde(d).ok())
                .collect(),
        )
    }
}

pub struct JSEnv {
    inner: JSEnvInterface,
    input_promise: Option<JsFuture>,
    input_buf: Vec<u8>,
    turt_helper: Option<TurtleRobotBox>,
}

impl AsyncWrite for JSEnv {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<f_io::Result<usize>> {
        if let Ok(s) = std::str::from_utf8(buf) {
            self.inner.write_output(s);
            Poll::Ready(Ok(s.len()))
        } else {
            Poll::Ready(Err(f_io::Error::new(f_io::ErrorKind::Other, "UTF-8 error")))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<f_io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<f_io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for JSEnv {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<f_io::Result<usize>> {
        while self.input_buf.len() < buf.len() {
            if self.input_promise.is_none() {
                // Call into JS
                let raw_promise = self.inner.read_input();
                self.input_promise = Some(JsFuture::from(raw_promise));
            }
            let fut = self.input_promise.as_mut().unwrap();
            match JsFuture::poll(Pin::new(fut), cx) {
                Poll::Pending => {
                    return Poll::Pending;
                }
                Poll::Ready(Err(_)) => {
                    self.input_promise = None;
                    return Poll::Ready(Err(f_io::Error::new(
                        f_io::ErrorKind::Other,
                        "JavaScript Error",
                    )));
                }
                Poll::Ready(Ok(js_str)) => {
                    self.input_promise = None;
                    match js_str.as_string() {
                        None => {
                            return Poll::Ready(Err(f_io::Error::new(
                                f_io::ErrorKind::Other,
                                "JavaScript type Error",
                            )))
                        }
                        Some(s) => {
                            // got a string from JS
                            if s.len() == 0 {
                                // EOF
                                break;
                            } else {
                                self.input_buf.extend_from_slice(s.as_ref());
                                // carry on with the while loop
                            }
                        }
                    }
                }
            }
        }

        // Copy to output
        if self.input_buf.len() < buf.len() {
            // hit EOF
            let input_len = self.input_buf.len();
            buf[..input_len].copy_from_slice(&self.input_buf);
            self.input_buf.clear();
            Poll::Ready(Ok(input_len))
        } else {
            buf.copy_from_slice(&self.input_buf[0..buf.len()]);
            let rest = self.input_buf.drain(buf.len()..).collect();
            self.input_buf = rest;
            Poll::Ready(Ok(buf.len()))
        }
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
    fn output_writer(&mut self) -> &mut (dyn AsyncWrite + Unpin) {
        self
    }

    fn input_reader(&mut self) -> &mut (dyn AsyncRead + Unpin) {
        self
    }

    fn warn(&mut self, msg: &str) {
        self.inner.warn(msg);
    }

    fn is_io_buffered(&self) -> bool {
        true
    }

    fn is_fingerprint_enabled(&self, fpr: i32) -> bool {
        safe_fingerprints().into_iter().any(|f| f == fpr) || fpr == string_to_fingerprint("TURT")
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

    fn fingerprint_support_library(&mut self, fpr: i32) -> Option<&mut dyn Any> {
        if fpr == string_to_fingerprint("TURT") {
            if self.turt_helper.is_none() {
                self.turt_helper = Some(SimpleRobot::new_in_box(TurtleDisplayWrapper {
                    display: self.inner.turtle_display(),
                }));
            }
            self.turt_helper.as_mut().map(|x| x as &mut dyn Any)
        } else {
            None
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
        let real_env = JSEnv {
            inner: env,
            input_promise: None,
            input_buf: vec![],
            turt_helper: None,
        };
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

    #[wasm_bindgen(js_name = "runAsync")]
    pub fn run_async(&mut self) -> js_sys::Promise {
        let self_ptr: *mut Self = self;
        wasm_bindgen_futures::future_to_promise(async move {
            let this: &mut Self = unsafe { &mut *self_ptr };
            let result = match this.interpreter.run_async(RunMode::Run).await {
                ProgramResult::Done(returncode) => returncode,
                _ => -1,
            };
            Ok(JsValue::from_f64(result as f64))
        })
    }

    #[wasm_bindgen(js_name = "runLimitedAsync")]
    pub fn run_limited_async(&mut self, loop_limit: u32) -> js_sys::Promise {
        let self_ptr: *mut Self = self;
        wasm_bindgen_futures::future_to_promise(async move {
            // Circumventing the borrow checker: the JS code promises not to
            // destroy the interpreter while it is running, and self lives at
            // a fixed place in memory. The compiler can't know these things.
            let this: &mut Self = unsafe { &mut *self_ptr };
            let result = match this
                .interpreter
                .run_async(RunMode::Limited(loop_limit))
                .await
            {
                ProgramResult::Done(returncode) => Some(returncode),
                ProgramResult::Panic => Some(-1),
                ProgramResult::Paused => None,
            };
            Ok(result
                .map(|n| JsValue::from_f64(n as f64))
                .unwrap_or_else(JsValue::null))
        })
    }

    #[wasm_bindgen(js_name = "stepAsync")]
    pub fn step_async(&mut self) -> js_sys::Promise {
        let self_ptr: *mut Self = self;
        wasm_bindgen_futures::future_to_promise(async move {
            // see comment in run_limited_async
            let this: &mut Self = unsafe { &mut *self_ptr };
            let result = match this.interpreter.run_async(RunMode::Step).await {
                ProgramResult::Done(returncode) => Some(returncode),
                ProgramResult::Panic => Some(-1),
                ProgramResult::Paused => None,
            };
            Ok(result
                .map(|n| JsValue::from_f64(n as f64))
                .unwrap_or_else(JsValue::null))
        })
    }

    #[wasm_bindgen(getter, js_name = "ipCount")]
    pub fn ip_count(&self) -> usize {
        self.interpreter.ips.len()
    }

    #[wasm_bindgen(js_name = "ipLocation")]
    pub fn ip_location(&self, ip_idx: usize) -> Option<Vec<i32>> {
        let loc = self.interpreter.ips.get(ip_idx)?.location;
        Some(vec![loc.x, loc.y])
    }

    #[wasm_bindgen(js_name = "ipDelta")]
    pub fn ip_delta(&self, ip_idx: usize) -> Option<Vec<i32>> {
        let d = self.interpreter.ips.get(ip_idx)?.delta;
        Some(vec![d.x, d.y])
    }

    #[wasm_bindgen(js_name = "projectedIpLocation")]
    pub fn projected_ip_location(&self, ip_idx: usize) -> Option<Vec<i32>> {
        let ip = self.interpreter.ips.get(ip_idx)?;
        let (next_loc, _) = self.interpreter.space.move_by(ip.location, ip.delta);
        Some(vec![next_loc.x, next_loc.y])
    }

    #[wasm_bindgen(js_name = "stackCount")]
    pub fn stack_count(&self, ip_idx: usize) -> usize {
        self.interpreter
            .ips
            .get(ip_idx)
            .map(|ip| ip.stack_stack.len())
            .unwrap_or(0)
    }

    /// Get a stack; TOSS is the stack_idx = 0
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
        let space = &self.interpreter.space;
        let mut start = space.min_idx().unwrap_or(bfvec(0, 0));
        start = bfvec(min(0, start.x), min(0, start.y));
        let end_incl = space.max_idx().unwrap_or(bfvec(0, 0));
        let size = bfvec(end_incl.x - start.x + 1, end_incl.y - start.y + 1);
        SrcIO::get_src_str(space, &start, &size, true)
    }

    #[wasm_bindgen(js_name = "getSrcLines")]
    pub fn get_src_lines(&self) -> Vec<JsValue> {
        let space = &self.interpreter.space;
        let mut start = space.min_idx().unwrap_or(bfvec(0, 0));
        start = bfvec(min(0, start.x), min(0, start.y));
        let end_incl = space.max_idx().unwrap_or(bfvec(0, 0));
        let line_len = end_incl.x - start.x + 1;

        (start.y..(end_incl.y + 1))
            .map(|y| SrcIO::get_src_str(space, &bfvec(start.x, y), &bfvec(line_len, 1), false))
            .map(|s| JsValue::from_str(&s))
            .collect()
    }
}
