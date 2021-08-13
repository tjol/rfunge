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
use std::io::Read;

use rfunge::{new_befunge_interpreter, read_befunge_bin, GenericEnv, IOMode};

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let filename = &argv[1];

    // Set up the interpreter
    let mut interpreter = new_befunge_interpreter::<i64, _>(GenericEnv {
        io_mode: IOMode::Text,
        output: std::io::stdout(),
        input: std::io::stdin(),
        warning_cb: |s: &str| eprintln!("{}", s.to_owned()),
    });

    // let src = std::fs::read_to_string(filename).unwrap();
    // read_befunge(&mut interpreter.space, &src);
    {
        let mut src = Vec::<u8>::new();
        File::open(filename)
            .and_then(|mut f| f.read_to_end(&mut src))
            .unwrap();
        read_befunge_bin(&mut interpreter.space, &src);
    }

    interpreter.run();
}
