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

//! This module contains only complex instructions; most instructions are
//! built into the interpreter

use std::cmp::Ordering;
use std::cmp::{max, min};
use std::future::Future;
use std::mem::size_of;
use std::pin::Pin;

use chrono::prelude::Utc;
use chrono::{Datelike, Timelike};
use num::ToPrimitive;
use pkg_version::{pkg_version_major, pkg_version_minor, pkg_version_patch};

use super::instruction_set::exec_instruction;
use super::motion::MotionCmds;
use super::{ExecMode, IOMode};
use super::{Funge, InstructionPointer, InstructionResult, InterpreterEnv};
use crate::fungespace::{FungeIndex, FungeSpace, FungeValue, SrcIO};

pub fn iterate<'a, F: Funge>(
    ip: &'a mut InstructionPointer<F>,
    space: &'a mut F::Space,
    env: &'a mut F::Env,
) -> Pin<Box<dyn Future<Output = InstructionResult> + 'a>> {
    Box::pin(async move {
        let n = ip.pop();
        let (mut new_loc, new_val_ref) = space.move_by(ip.location, ip.delta);
        let mut new_val = *new_val_ref;
        let mut loop_result = InstructionResult::Continue;
        let mut new_val_c = new_val.to_char();
        while new_val_c == ';' {
            // skip what must be skipped
            // fake-execute!
            let old_loc = ip.location;
            ip.location = new_loc;
            exec_instruction(new_val, ip, space, env).await;
            let (new_loc2, new_val_ref) = space.move_by(ip.location, ip.delta);
            new_loc = new_loc2;
            new_val = *new_val_ref;
            ip.location = old_loc;
            new_val_c = new_val.to_char();
        }
        if let Some(n) = n.to_usize() {
            if n == 0 {
                // surprising behaviour! 1k leads to the next instruction
                // being executed twice, 0k to it being skipped
                ip.location = new_loc;
                loop_result = InstructionResult::Continue;
            } else {
                let mut forks = 0;
                for _ in 0..n {
                    match exec_instruction(new_val, ip, space, env).await {
                        InstructionResult::Continue => {}
                        InstructionResult::Fork(n) => {
                            forks += n;
                            loop_result = InstructionResult::Fork(forks)
                        }
                        res => {
                            loop_result = res;
                            break;
                        }
                    }
                }
            }
        } else {
            // Reflect on over- or underflow
            ip.reflect();
        }
        loop_result
    })
}

pub fn begin_block<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    if let Some(n) = ip.pop().to_isize() {
        // take n items off the SOSS (old TOSS)
        let n_to_take = max(0, min(n, ip.stack().len() as isize));
        let zeros_for_toss = max(0, n - n_to_take);
        let zeros_for_soss = max(0, -n);

        let split_idx = ip.stack().len() - n_to_take as usize;
        let mut transfer_elems = ip.stack_mut().split_off(split_idx);

        for _ in 0..zeros_for_soss {
            ip.push(0.into());
        }

        let offset = ip.storage_offset;
        MotionCmds::push_vector(ip, offset); // onto SOSS / old TOSS

        // create a new stack
        ip.stack_stack.insert(0, Vec::new());

        for _ in 0..zeros_for_toss {
            ip.push(0.into());
        }

        ip.stack_mut().append(&mut transfer_elems);

        ip.storage_offset = ip.location + ip.delta;
    } else {
        ip.reflect();
    }

    InstructionResult::Continue
}

pub fn end_block<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    if ip.stack_stack.len() > 1 {
        if let Some(n) = ip.pop().to_isize() {
            let mut toss = ip.stack_stack.remove(0);

            // restore the storage offset
            ip.storage_offset = MotionCmds::pop_vector(ip);

            let n_to_take = max(0, min(n, toss.len() as isize));
            let zeros_for_soss = max(0, n - n_to_take);
            let n_to_pop = max(0, -n);

            if n_to_pop > 0 {
                for _ in 0..n_to_pop {
                    ip.pop();
                }
            } else {
                for _ in 0..zeros_for_soss {
                    ip.push(0.into());
                }

                let split_idx = toss.len() - n_to_take as usize;
                ip.stack_mut().append(&mut toss.split_off(split_idx));
            }
        } else {
            ip.reflect();
        }
    } else {
        ip.reflect();
    }

    InstructionResult::Continue
}

pub fn stack_under_stack<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let nstacks = ip.stack_stack.len();
    if nstacks > 1 {
        if let Some(n) = ip.pop().to_isize() {
            match n.cmp(&0) {
                Ordering::Greater => {
                    for _ in 0..n {
                        let v = ip.stack_stack[1].pop().unwrap_or_else(|| 0.into());
                        ip.push(v);
                    }
                }
                Ordering::Less => {
                    for _ in 0..(-n) {
                        let v = ip.pop();
                        ip.stack_stack[1].push(v);
                    }
                }
                Ordering::Equal => {}
            }
        } else {
            ip.reflect();
        }
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

pub fn input_file<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let filename = ip.pop_0gnirts();
    let flags = ip.pop();
    let dest = MotionCmds::pop_vector(ip);

    match env.get_iomode() {
        IOMode::Binary => {
            if let Ok(src) = env.read_file(&filename) {
                if flags & 1.into() == 1.into() {
                    // "binary mode" = linear mode
                    let mut dest = dest;
                    for b in src {
                        space[dest] = (b as i32).into();
                        dest = dest.one_further();
                    }
                } else {
                    // "text mode"
                    let size = F::Idx::read_bin_at(space, &dest, &src);
                    MotionCmds::push_vector(ip, size);
                    MotionCmds::push_vector(ip, dest);
                }
            } else {
                ip.reflect();
            }
        }
        IOMode::Text => {
            if let Some(src) = env
                .read_file(&filename)
                .ok()
                .and_then(|v| String::from_utf8(v).ok())
            {
                if flags & 1.into() == 1.into() {
                    // "binary mode" = linear mode
                    let mut dest = dest;
                    for c in src.chars() {
                        space[dest] = (c as i32).into();
                        dest = dest.one_further();
                    }
                } else {
                    // "text mode"
                    let size = F::Idx::read_str_at(space, &dest, &src);
                    MotionCmds::push_vector(ip, size);
                    MotionCmds::push_vector(ip, dest);
                }
            } else {
                ip.reflect();
            }
        }
    }

    InstructionResult::Continue
}

pub fn output_file<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let filename = ip.pop_0gnirts();
    let flags = ip.pop();
    let start = MotionCmds::pop_vector(ip);
    let size = MotionCmds::pop_vector(ip);

    let strip = (flags & 1.into()) == 1.into();

    if match env.get_iomode() {
        IOMode::Binary => {
            env.write_file(&filename, &F::Idx::get_src_bin(space, &start, &size, strip))
        }
        IOMode::Text => env.write_file(
            &filename,
            F::Idx::get_src_str(space, &start, &size, strip).as_bytes(),
        ),
    }
    .is_err()
    {
        ip.reflect();
    }

    InstructionResult::Continue
}

pub fn execute<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    if env.have_execute() == ExecMode::Disabled {
        ip.reflect();
    } else {
        let cmd = ip.pop_0gnirts();
        ip.push(env.execute_command(&cmd).into());
    }

    InstructionResult::Continue
}

pub fn sysinfo<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    env: &mut F::Env,
) -> InstructionResult {
    let mut sysinfo_cells = Vec::<F::Value>::new();
    // what should we push?
    let n = ip.pop();
    let exec_flag = env.have_execute();
    // Set everything up first

    // 1. flags
    let mut impl_flags = 0x1; // concurrent funge-98
    if env.have_file_input() {
        impl_flags |= 0x2
    }
    if env.have_file_output() {
        impl_flags |= 0x4
    }
    if exec_flag != ExecMode::Disabled {
        impl_flags |= 0x8
    }
    if !env.is_io_buffered() {
        impl_flags |= 0x10
    }
    sysinfo_cells.push(impl_flags.into());

    // 2. size of cell
    sysinfo_cells.push((size_of::<F::Value>() as i32).into());

    // 3. handprint
    sysinfo_cells.push(env.handprint().into());

    // 4. version number
    sysinfo_cells.push(
        (pkg_version_major!() * 1000000 + pkg_version_minor!() * 1000 + pkg_version_patch!())
            .into(),
    );

    // 5. "operating paradigm"
    sysinfo_cells.push(
        match exec_flag {
            ExecMode::Disabled => 0,
            ExecMode::System => 1,
            ExecMode::SpecificShell => 2,
            ExecMode::SameShell => 3,
        }
        .into(),
    );

    // 6. path separator character
    sysinfo_cells.push((std::path::MAIN_SEPARATOR as i32).into());

    // 7. numer of scalars per vector
    sysinfo_cells.push(F::Idx::RANK.into());

    // 8. IP ID
    sysinfo_cells.push(ip.id);

    // 9. IP team number
    sysinfo_cells.push(0.into());

    // 10. Position
    let mut tmp_vec = Vec::new();
    F::Idx::push_vector_onto(&mut tmp_vec, ip.location);
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    // 11. Delta
    let mut tmp_vec = Vec::new();
    F::Idx::push_vector_onto(&mut tmp_vec, ip.delta);
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    // 12. Storage offset
    let mut tmp_vec = Vec::new();
    F::Idx::push_vector_onto(&mut tmp_vec, ip.storage_offset);
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    let idx: F::Value = (sysinfo_cells.len() as i32).into();
    // Only calculate the next bit if we need it as it's quite expensive
    if n <= 0.into() || (n > idx && n <= idx + (2 * F::Idx::RANK).into()) {
        // 13. Least point

        let mut tmp_vec = Vec::new();
        let least_idx = space.min_idx().unwrap_or_else(F::Idx::origin);
        F::Idx::push_vector_onto(&mut tmp_vec, least_idx);
        sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

        // 14. Greatest point

        let mut tmp_vec = Vec::new();
        F::Idx::push_vector_onto(
            &mut tmp_vec,
            space.max_idx().unwrap_or_else(F::Idx::origin) - least_idx,
        );
        sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());
    } else {
        F::Idx::push_vector_onto(&mut sysinfo_cells, F::Idx::origin());
        F::Idx::push_vector_onto(&mut sysinfo_cells, F::Idx::origin());
    }

    // 15 & 16: Time
    let datetime = Utc::now();

    // 15. ((year - 1900) * 256 * 256) + (month * 256) + (day of month)
    sysinfo_cells.push(
        (((datetime.year() - 1900) * 256 * 256)
            + (datetime.month() as i32 * 256)
            + datetime.day() as i32)
            .into(),
    );

    // 16. (hour * 256 * 256) + (minute * 256) + (second)
    sysinfo_cells.push(
        ((datetime.hour() as i32 * 256 * 256)
            + (datetime.minute() as i32 * 256)
            + datetime.second() as i32)
            .into(),
    );

    // 17. size of stack stack
    sysinfo_cells.push((ip.stack_stack.len() as i32).into());

    // 18. sizes of stacks
    for stack in ip.stack_stack.iter() {
        sysinfo_cells.push((stack.len() as i32).into());
    }

    // 19. command line args
    for arg in env.argv().into_iter() {
        for c in arg.chars() {
            sysinfo_cells.push((c as i32).into());
        }
        sysinfo_cells.push(0.into());
    }
    sysinfo_cells.push(0.into());
    sysinfo_cells.push(0.into());

    // 20. environment
    for (key, value) in env.env_vars().into_iter() {
        let s = format!("{}={}", key, value);
        for c in s.chars() {
            sysinfo_cells.push((c as i32).into());
        }
        sysinfo_cells.push(0.into());
    }
    sysinfo_cells.push(0.into());
    sysinfo_cells.push(0.into());

    if n > (sysinfo_cells.len() as i32).into() {
        // pick one pre-sysinfo cell
        let pick_n = n - (sysinfo_cells.len() as i32).into();
        let idx = ip.stack().len() as isize - pick_n.to_isize().unwrap();
        if idx >= 0 {
            ip.push(ip.stack()[idx as usize]);
        }
    } else if n > 0.into() {
        // pick one cell from sysinfo
        ip.push(sysinfo_cells[n.to_usize().unwrap() - 1]);
    } else {
        // push it all
        for cell in sysinfo_cells.into_iter().rev() {
            ip.push(cell);
        }
    }

    InstructionResult::Continue
}
