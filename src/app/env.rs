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

use std::any::Any;
use std::fs::File;
use std::io::{stderr, Error, ErrorKind, Read, Result, Write};
use std::process::Command;

use async_std::io::{stdin, stdout, Stdin, Stdout};
use futures_lite::io::{AsyncRead, AsyncWrite};

use rfunge::interpreter::fingerprints::{
    string_to_fingerprint,
    TURT::{SimpleRobot, TurtleRobotBox},
};
use rfunge::{all_fingerprints, safe_fingerprints, ExecMode, IOMode, InterpreterEnv};

use super::turt::LocalTurtDisplay;

pub struct CmdLineEnv {
    io_mode: IOMode,
    warnings: bool,
    sandbox: bool,
    stdout: Stdout,
    stdin: Stdin,
    argv: Vec<String>,
    allowed_fingerprints: Vec<i32>,
    turt_helper: Option<TurtleRobotBox>,
}

impl CmdLineEnv {
    pub fn new(io_mode: IOMode, warnings: bool, sandbox: bool, argv: Vec<String>) -> Self {
        Self {
            io_mode,
            warnings,
            stdout: stdout(),
            stdin: stdin(),
            sandbox,
            argv,
            allowed_fingerprints: if sandbox {
                safe_fingerprints()
            } else {
                all_fingerprints()
            },
            turt_helper: None,
        }
    }

    pub fn init_turt(&mut self, disp: LocalTurtDisplay) {
        self.turt_helper = Some(SimpleRobot::new_in_box(disp));
    }
}

impl InterpreterEnv for CmdLineEnv {
    fn get_iomode(&self) -> IOMode {
        self.io_mode
    }
    fn is_io_buffered(&self) -> bool {
        true
    }
    fn output_writer(&mut self) -> &mut (dyn AsyncWrite + Unpin) {
        &mut self.stdout
    }
    fn input_reader(&mut self) -> &mut (dyn AsyncRead + Unpin) {
        &mut self.stdin
    }
    fn warn(&mut self, msg: &str) {
        if self.warnings {
            writeln!(stderr(), "{}", msg).ok();
        }
    }
    fn have_file_input(&self) -> bool {
        !self.sandbox
    }
    fn have_file_output(&self) -> bool {
        !self.sandbox
    }
    fn have_execute(&self) -> ExecMode {
        if self.sandbox {
            ExecMode::Disabled
        } else {
            ExecMode::System
        }
    }
    fn read_file(&mut self, filename: &str) -> Result<Vec<u8>> {
        if self.sandbox {
            Err(Error::from(ErrorKind::PermissionDenied))
        } else {
            let mut buf = Vec::new();
            File::open(filename).and_then(|mut f| f.read_to_end(&mut buf))?;
            Ok(buf)
        }
    }
    fn write_file(&mut self, filename: &str, content: &[u8]) -> Result<()> {
        if self.sandbox {
            Err(Error::from(ErrorKind::PermissionDenied))
        } else {
            File::create(filename).and_then(|mut f| f.write_all(content))
        }
    }
    fn execute_command(&mut self, command: &str) -> i32 {
        if self.sandbox {
            -1
        } else if cfg!(unix) {
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .status()
                .ok()
                .and_then(|s| s.code())
                .unwrap_or(-1)
        } else if cfg!(windows) {
            Command::new("CMD")
                .arg("/C")
                .arg(command)
                .status()
                .ok()
                .and_then(|s| s.code())
                .unwrap_or(-1)
        } else {
            eprintln!(
                "WARNING: Attempted to execute command, but I don't know how on this system!"
            );
            -1
        }
    }
    fn env_vars(&mut self) -> Vec<(String, String)> {
        if self.sandbox {
            Vec::new()
        } else {
            std::env::vars_os()
                .into_iter()
                .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
                .collect()
        }
    }
    fn argv(&mut self) -> Vec<String> {
        self.argv.clone()
    }
    fn is_fingerprint_enabled(&self, fpr: i32) -> bool {
        self.allowed_fingerprints.iter().any(|f| *f == fpr)
    }

    fn fingerprint_support_library(&mut self, fpr: i32) -> Option<&mut dyn Any> {
        if fpr == string_to_fingerprint("TURT") {
            if self.turt_helper.is_none() {
                self.turt_helper = Some(SimpleRobot::new_in_box(LocalTurtDisplay::new()));
            }
            self.turt_helper.as_mut().map(|x| x as &mut dyn Any)
        } else {
            None
        }
    }
}
