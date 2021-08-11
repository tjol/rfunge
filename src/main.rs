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

use rfunge::{IOMode, InstructionPointer, Interpreter, InterpreterEnvironment, PagedFungeSpace};
use rfunge::{bfvec, read_befunge_bin, BefungeVec};

fn main() {
    let argv: Vec<String> = std::env::args().collect();

    let filename = &argv[1];

    // let src = std::fs::read_to_string(filename).unwrap();

    // // Set up the interpreter
    // let mut interpreter = Interpreter {
    //     ips: vec![InstructionPointer::new()],
    //     space: PagedFungeSpace::<i64, i64>::new_with_page_size(128),
    // };

    // read_unefunge(&mut interpreter.space, &src);

    // Set up the interpreter
    let mut interpreter = Interpreter {
        ips: vec![InstructionPointer::new()],
        space: PagedFungeSpace::<BefungeVec<i64>, i64>::new_with_page_size(bfvec(80, 25)),
        env: InterpreterEnvironment {
            output: Box::new(std::io::stdout()),
            input: Some(Box::new(std::io::stdin())),
            warn: Some(Box::new(|s| eprintln!("{}", s))),
            io_mode: IOMode::Text,
        },
    };

    // read_befunge(&mut interpreter.space, &src);
    {
        let mut src_file = std::fs::File::open(filename).unwrap();
        read_befunge_bin(&mut interpreter.space, &mut src_file).unwrap();
    }

    interpreter.run();
}
