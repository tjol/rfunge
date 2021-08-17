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

#![cfg(not(target_arch = "wasm32"))]

use crate::{
    new_befunge_interpreter, read_funge_src, read_funge_src_bin, BefungeVec, IOMode, Interpreter,
    InterpreterEnv, PagedFungeSpace, ProgramResult, RunMode,
};

// --------------------------------------------------------
// C API
// --------------------------------------------------------

use std::boxed::Box;
use std::ffi::c_void;
use std::io;
use std::io::{Read, Write};

type CWriteFn = unsafe extern "C" fn(*const u8, usize, *mut c_void) -> isize;
type CReadFn = unsafe extern "C" fn(*mut u8, usize, *mut c_void) -> isize;

pub struct CAPIEnv {
    is_unicode: bool,
    write_cb: Option<CWriteFn>,
    read_cb: Option<CReadFn>,
    warn_cb: Option<CWriteFn>,
    user_data: *mut c_void,
}

impl Write for CAPIEnv {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(cb) = self.write_cb {
            let c_result = unsafe { cb(buf.as_ptr(), buf.len(), self.user_data) };
            if c_result < 0 {
                Err(io::Error::new(io::ErrorKind::Other, "FFI error"))
            } else {
                Ok(c_result as usize)
            }
        } else {
            // act as a sink
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for CAPIEnv {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(cb) = self.read_cb {
            let c_result = unsafe { cb(buf.as_mut_ptr(), buf.len(), self.user_data) };
            if c_result < 0 {
                Err(io::Error::new(io::ErrorKind::Other, "FFI error"))
            } else {
                Ok(c_result as usize)
            }
        } else {
            // EOF by default
            Ok(0)
        }
    }
}

impl InterpreterEnv for CAPIEnv {
    fn get_iomode(&self) -> IOMode {
        if self.is_unicode {
            IOMode::Text
        } else {
            IOMode::Binary
        }
    }
    fn output_writer(&mut self) -> &mut dyn Write {
        self
    }

    fn input_reader(&mut self) -> &mut dyn Read {
        self
    }

    fn warn(&mut self, msg: &str) {
        let mut msg = msg.to_owned();
        msg.push('\n');
        if let Some(cb) = self.warn_cb {
            unsafe {
                cb(msg.as_ptr(), msg.len(), self.user_data);
            }
        }
    }

    fn is_io_buffered(&self) -> bool {
        true
    }
}

type RFungeBefungeInterp =
    Interpreter<BefungeVec<i32>, PagedFungeSpace<BefungeVec<i32>, i32>, CAPIEnv>;

#[no_mangle]
pub extern "C" fn rfunge_new_befunge_interpreter(
    unicode_mode: bool,
    out_cb: Option<CWriteFn>,
    in_cb: Option<CReadFn>,
    err_cb: Option<CWriteFn>,
    user_data: *mut c_void,
) -> *mut RFungeBefungeInterp {
    Box::into_raw(Box::new(new_befunge_interpreter::<i32, _>(CAPIEnv {
        is_unicode: unicode_mode,
        write_cb: out_cb,
        read_cb: in_cb,
        warn_cb: err_cb,
        user_data,
    })))
}

#[no_mangle]
pub extern "C" fn rfunge_free_interpreter(interp: *mut RFungeBefungeInterp) {
    unsafe {
        Box::from_raw(interp);
    }
}

#[no_mangle]
pub extern "C" fn rfunge_load_src(
    interp: *mut RFungeBefungeInterp,
    buf: *const u8,
    len: usize,
) -> bool {
    let interp_ref = unsafe { &mut (*interp) };
    let src_bin = unsafe { std::slice::from_raw_parts(buf, len) };

    if interp_ref.env.is_unicode {
        if let Ok(src) = std::str::from_utf8(src_bin) {
            read_funge_src(&mut interp_ref.space, src);
            true
        } else {
            false
        }
    } else {
        read_funge_src_bin(&mut interp_ref.space, src_bin);
        true
    }
}

#[no_mangle]
pub extern "C" fn rfunge_run(interp: *mut RFungeBefungeInterp) -> i32 {
    match unsafe { &mut (*interp) }.run(RunMode::Run) {
        ProgramResult::Done(returncode) => returncode,
        _ => -1,
    }
}
