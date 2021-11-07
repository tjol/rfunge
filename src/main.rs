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

mod app;

use std::fs::File;
use std::io::Read;

use clap::{App, Arg};
use regex::Regex;

use rfunge::fungespace::SrcIO;
use rfunge::interpreter::MotionCmds;
#[cfg(not(feature = "turt-gui"))]
use rfunge::RunMode;
use rfunge::{
    new_befunge_interpreter, new_unefunge_interpreter, read_funge_src, read_funge_src_bin, Funge,
    FungeSpace, FungeValue, IOMode, Interpreter, ProgramResult,
};

use app::env::CmdLineEnv;

#[cfg(feature = "turt-gui")]
use app::turt::run_with_turt;

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
            Arg::with_name("32bit")
                .short("I")
                .long("32bit")
                .help("32-bit mode")
                .display_order(4),
        )
        .arg(
            Arg::with_name("64bit")
                .short("L")
                .long("64bit")
                .help("64-bit mode (default)")
                .conflicts_with("32bit")
                .display_order(4),
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
    let mut argv = vec![filename.to_owned()];
    argv.append(&mut arg_matches.values_of_lossy("ARGS").unwrap_or_default());
    let sandbox = arg_matches.is_present("sandbox");
    let show_warnings = arg_matches.is_present("warn");

    let make_env = move || {
        CmdLineEnv::new(
            if is_unicode {
                IOMode::Text
            } else {
                IOMode::Binary
            },
            show_warnings,
            sandbox,
            argv,
        )
    };

    let is_32bit = arg_matches.is_present("32bit");

    let result = if dim == 1 {
        // unefunge
        if is_32bit {
            read_and_run(
                move || new_unefunge_interpreter::<i32, _>(make_env()),
                src_bin,
                is_unicode,
            )
        } else {
            read_and_run(
                move || new_unefunge_interpreter::<i64, _>(make_env()),
                src_bin,
                is_unicode,
            )
        }
    } else if dim == 2 {
        // befunge
        if is_32bit {
            read_and_run(
                move || new_befunge_interpreter::<i32, _>(make_env()),
                src_bin,
                is_unicode,
            )
        } else {
            read_and_run(
                move || new_befunge_interpreter::<i64, _>(make_env()),
                src_bin,
                is_unicode,
            )
        }
    } else {
        ProgramResult::Panic
    };

    std::process::exit(match result {
        ProgramResult::Done(returncode) => returncode,
        _ => 1,
    });
}

fn read_and_run<Idx, Space, InitFn>(
    make_interpreter: InitFn,
    src_bin: Vec<u8>,
    is_unicode: bool,
) -> ProgramResult
where
    Idx: MotionCmds<Space, CmdLineEnv> + SrcIO<Space>,
    Space: FungeSpace<Idx> + 'static,
    Space::Output: FungeValue,
    InitFn: FnOnce() -> Interpreter<Idx, Space, CmdLineEnv> + Send + 'static,
{
    run::<_, Interpreter<Idx, Space, CmdLineEnv>>(move || {
        let mut interpreter = make_interpreter();
        if is_unicode {
            let src_str = String::from_utf8(src_bin).unwrap();
            read_funge_src(interpreter.space.as_mut().unwrap(), &src_str);
        } else {
            read_funge_src_bin(interpreter.space.as_mut().unwrap(), &src_bin);
        }
        interpreter
    })
}

#[cfg(not(feature = "turt-gui"))]
pub fn run<InitFn, Interp>(make_interpreter: InitFn) -> ProgramResult
where
    InitFn: FnOnce() -> Interpreter<Interp::Idx, Interp::Space, Interp::Env> + Send + 'static,
    Interp: Funge<Env = CmdLineEnv> + 'static,
{
    let mut interpreter = make_interpreter();
    interpreter.run(RunMode::Run)
}

#[cfg(feature = "turt-gui")]
pub fn run<InitFn, Interp>(make_interpreter: InitFn) -> ProgramResult
where
    InitFn: FnOnce() -> Interpreter<Interp::Idx, Interp::Space, Interp::Env> + Send + 'static,
    Interp: Funge<Env = CmdLineEnv> + 'static,
{
    run_with_turt::<InitFn, Interp>(make_interpreter)
}
