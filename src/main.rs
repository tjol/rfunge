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

use regex::Regex;
use std::fs::File;
use std::io::{Read, Write};

use clap::{App, Arg};

use rfunge::{
    new_befunge_interpreter, new_unefunge_interpreter, read_befunge, read_befunge_bin,
    read_unefunge, read_unefunge_bin, GenericEnv, IOMode,
};

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
    let mut warning_stream: Box<dyn Write> = if arg_matches.is_present("warn") {
        Box::new(std::io::stderr())
    } else {
        Box::new(std::io::sink())
    };
    let env = GenericEnv {
        io_mode: if is_unicode {
            IOMode::Text
        } else {
            IOMode::Binary
        },
        output: std::io::stdout(),
        input: std::io::stdin(),
        warning_cb: |s: &str| writeln!(warning_stream, "{}", s.to_owned()).unwrap(),
    };

    if dim == 1 {
        // unefunge
        let mut interpreter = new_unefunge_interpreter::<i64, _>(env);
        if is_unicode {
            let src_str = String::from_utf8(src_bin).unwrap();
            read_unefunge(&mut interpreter.space, &src_str)
        } else {
            read_unefunge_bin(&mut interpreter.space, &src_bin);
        }
        interpreter.run();
    } else if dim == 2 {
        // befunge
        let mut interpreter = new_befunge_interpreter::<i64, _>(env);
        if is_unicode {
            let src_str = String::from_utf8(src_bin).unwrap();
            read_befunge(&mut interpreter.space, &src_str)
        } else {
            read_befunge_bin(&mut interpreter.space, &src_bin);
        }
        interpreter.run();
    }
}
