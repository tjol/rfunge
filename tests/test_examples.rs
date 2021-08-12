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

use colored::Colorize;
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rfunge::{
    new_befunge_interpreter, read_befunge_bin, IOMode, InterpreterEnvironment, ProgramResult,
};

const TEST_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");

fn get_b98_tests() -> io::Result<Vec<(PathBuf, PathBuf)>> {
    let mut test_cases = HashMap::new();
    let mut expected_out_files = HashMap::new();
    let test_cases_dir = Path::new(TEST_ROOT).join("test_cases");
    for entry in read_dir(test_cases_dir)? {
        let p = entry?.path();
        let fname = p.file_name();
        if let Some(fname) = fname.and_then(|n| n.to_str()) {
            let mut fname = fname.to_owned();
            if fname.ends_with(".b98") {
                test_cases.insert(fname, p);
            } else if fname.ends_with(".b98.expected") {
                fname.truncate(fname.len() - ".expected".len());
                expected_out_files.insert(fname, p);
            }
        }
    }

    let mut result: Vec<(PathBuf, PathBuf)> = Vec::new();

    // See what matches up
    for (tc, p) in test_cases.iter() {
        if let Some(expected_p) = expected_out_files.get(tc) {
            result.push((p.to_path_buf(), expected_p.to_path_buf()));
        }
    }

    return Ok(result);
}

fn run_b98_test(program_path: &Path, output_path: &Path) {
    let program_name = program_path.file_name().unwrap().to_string_lossy();
    eprint!("befunge test {} ... ", program_name);
    io::stderr().flush().unwrap();

    let mut output = Vec::<u8>::new();
    let mut warn_log = Vec::<String>::new();
    {
        // Set up the interpreter
        let mut interpreter = new_befunge_interpreter::<i32, _, _, _>(InterpreterEnvironment {
            output: &mut output,
            input: io::empty(),
            warn: (|s| warn_log.push(s.to_owned())),
            io_mode: IOMode::Binary,
        });

        {
            let mut src_file = File::open(program_path).unwrap();
            read_befunge_bin(&mut interpreter.space, &mut src_file).unwrap();
        }

        assert_eq!(interpreter.run(), ProgramResult::Ok);
    }
    let mut ref_out: Vec<u8> = vec![0; output.len()];
    {
        let mut ref_file = File::open(output_path).unwrap();
        let read_result = ref_file.read(&mut ref_out);
        assert!(matches!(read_result, Ok(_)));
        assert_eq!(read_result.unwrap(), output.len());
        let mut test_buf: Vec<u8> = vec![0; 1];
        assert!(matches!(ref_file.read(&mut test_buf), Ok(0))); // check EOF
    }
    assert_eq!(output, ref_out);
    eprintln!("{}", "ok".green());
}

fn main() {
    let test_fns = get_b98_tests().unwrap();
    for (test_path, result_path) in test_fns {
        run_b98_test(&test_path, &result_path);
    }
}
