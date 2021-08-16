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
use std::io::{Empty, Read, Write};
use std::path::{Path, PathBuf};

use rfunge::{
    new_befunge_interpreter, read_funge_src_bin, ExecMode, IOMode, InterpreterEnv, ProgramResult,
    RunMode,
};

struct TestEnv {
    output: Vec<u8>,
    input: Empty,
    working_dir: PathBuf,
}

impl InterpreterEnv for TestEnv {
    fn get_iomode(&self) -> IOMode {
        IOMode::Binary
    }
    fn is_io_buffered(&self) -> bool {
        true
    }
    fn output_writer(&mut self) -> &mut dyn Write {
        &mut self.output
    }
    fn input_reader(&mut self) -> &mut dyn Read {
        &mut self.input
    }
    fn warn(&mut self, _msg: &str) {}
    fn have_file_input(&self) -> bool {
        true
    }
    fn have_execute(&self) -> ExecMode {
        ExecMode::Disabled
    }
    fn read_file(&mut self, filename: &str) -> io::Result<Vec<u8>> {
        let filepath = self.working_dir.join(filename);
        let mut buf = Vec::new();
        File::open(filepath).and_then(|mut f| f.read_to_end(&mut buf))?;
        Ok(buf)
    }
}

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
    let dir_name = program_path.parent().unwrap();
    eprint!("befunge test {} ... ", program_name);
    io::stderr().flush().unwrap();

    let output = {
        // Set up the interpreter
        let mut interpreter = new_befunge_interpreter::<i32, _>(TestEnv {
            output: Vec::new(),
            input: std::io::empty(),
            working_dir: dir_name.to_owned(),
        });

        {
            let mut src = Vec::<u8>::new();
            File::open(program_path)
                .and_then(|mut f| f.read_to_end(&mut src))
                .unwrap();
            read_funge_src_bin(&mut interpreter.space, &src);
        }

        assert_eq!(interpreter.run(RunMode::Run), ProgramResult::Done(0));

        interpreter.env.output
    };
    let mut ref_out = Vec::<u8>::new();
    File::open(output_path)
        .and_then(|mut f| f.read_to_end(&mut ref_out))
        .unwrap();
    assert_eq!(output, ref_out);
    eprintln!("{}", "ok".green());
}

fn main() {
    let test_fns = get_b98_tests().unwrap();
    for (test_path, result_path) in test_fns {
        run_b98_test(&test_path, &result_path);
    }
}
