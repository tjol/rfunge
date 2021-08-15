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

use std::fs::File;
use std::io;
use std::io::{stderr, stdin, stdout, Read, Stdin, Stdout, Write};
use std::process::Command;

use clap::{App, Arg};
use regex::Regex;

use rfunge::{
    new_befunge_interpreter, new_unefunge_interpreter, read_funge_src, read_funge_src_bin,
    ExecMode, IOMode, InterpreterEnv,
};

struct CmdLineEnv {
    io_mode: IOMode,
    warnings: bool,
    sandbox: bool,
    stdout: Stdout,
    stdin: Stdin,
}

impl InterpreterEnv for CmdLineEnv {
    fn get_iomode(&self) -> IOMode {
        self.io_mode
    }
    fn is_io_buffered(&self) -> bool {
        true
    }
    fn output_writer(&mut self) -> &mut dyn Write {
        &mut self.stdout
    }
    fn input_reader(&mut self) -> &mut dyn Read {
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
    fn read_file(&mut self, filename: &str) -> io::Result<Vec<u8>> {
        if self.sandbox {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        } else {
            let mut buf = Vec::new();
            File::open(filename).and_then(|mut f| f.read_to_end(&mut buf))?;
            Ok(buf)
        }
    }
    fn write_file(&mut self, filename: &str, content: &[u8]) -> io::Result<()> {
        if self.sandbox {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        } else {
            File::create(filename).and_then(|mut f| f.write_all(content))
        }
    }
    fn execute_command(&mut self, command: &str) -> i32 {
        if self.sandbox {
            -1
        } else {
            if cfg!(unix) {
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
    }
}

fn main() {
    let arg_matches = App::new(env!("CARGO_BIN_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about("Funge-98 interpreter")
        .arg(
            Arg::with_name("warn")
                .short("w")
                .long("warn")
                .help("Enable warnings")
                .display_order(4),
        )
        .arg(
            Arg::with_name("binary")
                .short("b")
                .long("binary")
                .help("Binary mode (default)")
                .display_order(3),
        )
        .arg(
            Arg::with_name("unicode")
                .short("u")
                .long("unicode")
                .help("Unicode mode")
                .conflicts_with("binary")
                .display_order(3),
        )
        .arg(
            Arg::with_name("sandbox")
                .short("s")
                .long("sandbox")
                .help("Run in sandbox / secure mode"),
        )
        .arg(
            Arg::with_name("unefunge")
                .short("1")
                .long("unefunge")
                .help("Unefunge mode")
                .display_order(1),
        )
        .arg(
            Arg::with_name("befunge")
                .short("2")
                .long("befunge")
                .help("Befunge mode")
                .conflicts_with("unefunge")
                .display_order(2),
        )
        .arg(
            Arg::with_name("PROGRAM")
                .help("Funge-98 source to execute")
                .required(true),
        )
        .arg(
            Arg::with_name("ARGS")
                .help("Arguments to pass to program")
                .required(false)
                .multiple(true),
        )
        .get_matches();

    let filename = arg_matches.value_of("PROGRAM").unwrap();

    let unefunge_fn_re = Regex::new(r"(?i)\.u(f|98|nefunge)$").unwrap();
    let befunge_fn_re = Regex::new(r"(?i)\.b(f|98|efunge)$").unwrap();
    // Is this Unefunge or Befunge?
    let dim = if arg_matches.is_present("unefunge") {
        1
    } else if arg_matches.is_present("befunge") {
        2
    } else if unefunge_fn_re.is_match(filename) {
        1
    } else if befunge_fn_re.is_match(filename) {
        2
    } else {
        0
    };
    if dim == 0 {
        eprintln!(
            "ERROR: Can't tell if this is unefunge or befunge. Try specifying the option -1 or -2!"
        );
        std::process::exit(2);
    }

    // Read the program source
    let mut src_bin = Vec::<u8>::new();
    if filename == "-" {
        std::io::stdin().read_to_end(&mut src_bin)
    } else {
        File::open(filename).and_then(|mut f| f.read_to_end(&mut src_bin))
    }
    .unwrap();

    let is_unicode = arg_matches.is_present("unicode");

    // Set up the interpreter
    let env = CmdLineEnv {
        io_mode: if is_unicode {
            IOMode::Text
        } else {
            IOMode::Binary
        },
        warnings: arg_matches.is_present("warn"),
        sandbox: arg_matches.is_present("sandbox"),
        stdout: stdout(),
        stdin: stdin(),
    };

    if dim == 1 {
        // unefunge
        let mut interpreter = new_unefunge_interpreter::<i64, _>(env);
        if is_unicode {
            let src_str = String::from_utf8(src_bin).unwrap();
            read_funge_src(&mut interpreter.space, &src_str)
        } else {
            read_funge_src_bin(&mut interpreter.space, &src_bin);
        }
        interpreter.run();
    } else if dim == 2 {
        // befunge
        let mut interpreter = new_befunge_interpreter::<i64, _>(env);
        if is_unicode {
            let src_str = String::from_utf8(src_bin).unwrap();
            read_funge_src(&mut interpreter.space, &src_str)
        } else {
            read_funge_src_bin(&mut interpreter.space, &src_bin);
        }
        interpreter.run();
    }
}
