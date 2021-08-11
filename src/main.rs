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

use rfunge::{new_befunge_interpreter, read_befunge_bin, IOMode, InterpreterEnvironment};

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let filename = &argv[1];

    // Set up the interpreter
    let mut interpreter = new_befunge_interpreter::<i64, _, _, _>(InterpreterEnvironment {
        output: std::io::stdout(),
        input: std::io::stdin(),
        warn: (|s| eprintln!("{}", s)),
        io_mode: IOMode::Text,
    });

    // let src = std::fs::read_to_string(filename).unwrap();
    // read_befunge(&mut interpreter.space, &src);
    {
        let mut src_file = std::fs::File::open(filename).unwrap();
        read_befunge_bin(&mut interpreter.space, &mut src_file).unwrap();
    }

    interpreter.run();
}
