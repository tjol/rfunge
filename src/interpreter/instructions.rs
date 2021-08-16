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

use std::cmp::{max, min};
use std::mem::size_of;

use chrono::{Datelike, NaiveDateTime, Timelike};
use num::ToPrimitive;
use pkg_version::{pkg_version_major, pkg_version_minor, pkg_version_patch};

use super::instruction_set::exec_instruction;
use super::ip::InstructionPointer;
use super::motion::MotionCmds;
use super::{ExecMode, IOMode};
use super::{InstructionResult, InterpreterEnv};
use crate::fungespace::{FungeSpace, FungeValue, SrcIO};

pub fn iterate<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let n = ip.pop();
    let (mut new_loc, new_val_ref) = space.move_by(ip.location, ip.delta);
    let mut new_val = *new_val_ref;
    let mut loop_result = InstructionResult::Continue;
    if let Some(n) = n.to_isize() {
        if n <= 0 {
            // surprising behaviour! 1k leads to the next instruction
            // being executed twice, 0k to it being skipped
            ip.location = new_loc;
            loop_result = InstructionResult::Continue;
        } else {
            let mut new_val_c = new_val.to_char();
            while new_val_c == ';' {
                // skip what must be skipped
                // fake-execute!
                let old_loc = ip.location;
                ip.location = new_loc;
                exec_instruction(new_val, ip, space, env);
                let (new_loc2, new_val_ref) = space.move_by(ip.location, ip.delta);
                new_loc = new_loc2;
                new_val = *new_val_ref;
                ip.location = old_loc;
                new_val_c = new_val.to_char();
            }
            for _ in 0..n {
                match exec_instruction(new_val, ip, space, env) {
                    InstructionResult::Continue => {}
                    res => {
                        loop_result = res;
                        break;
                    }
                }
            }
        }
    } else {
        // Reflect on overflow
        ip.reflect();
    }
    loop_result
}

pub fn begin_block<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
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

        MotionCmds::push_vector(ip, ip.storage_offset); // onto SOSS / old TOSS

        // create a new stack
        ip.stack_stack.push(Vec::new());

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

pub fn end_block<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    if ip.stack_stack.len() > 1 {
        if let Some(n) = ip.pop().to_isize() {
            let mut toss = ip.stack_stack.pop().unwrap();

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

pub fn stack_under_stack<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    _env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let nstacks = ip.stack_stack.len();
    if nstacks > 1 {
        if let Some(n) = ip.pop().to_isize() {
            if n > 0 {
                for _ in 0..n {
                    let v = ip.stack_stack[nstacks - 2].pop().unwrap_or(0.into());
                    ip.push(v);
                }
            } else if n < 0 {
                for _ in 0..(-n) {
                    let v = ip.pop();
                    ip.stack_stack[nstacks - 2].push(v);
                }
            }
        } else {
            ip.reflect();
        }
    } else {
        ip.reflect();
    }
    InstructionResult::Continue
}

pub fn input_file<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
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
                    let size = Idx::read_bin_at(space, &dest, &src);
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
                    let size = Idx::read_str_at(space, &dest, &src);
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

pub fn output_file<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let filename = ip.pop_0gnirts();
    let flags = ip.pop();
    let start = MotionCmds::pop_vector(ip);
    let size = MotionCmds::pop_vector(ip);

    let strip = (flags & 1.into()) == 1.into();

    if match env.get_iomode() {
        IOMode::Binary => env.write_file(&filename, &Idx::get_src_bin(space, &start, &size, strip)),
        IOMode::Text => env.write_file(
            &filename,
            Idx::get_src_str(space, &start, &size, strip).as_bytes(),
        ),
    }
    .is_err()
    {
        ip.reflect();
    }

    InstructionResult::Continue
}

pub fn execute<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    _space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    if env.have_execute() == ExecMode::Disabled {
        ip.reflect();
    } else {
        let cmd = ip.pop_0gnirts();
        ip.push(env.execute_command(&cmd).into());
    }

    InstructionResult::Continue
}

pub fn sysinfo<Idx, Space, Env>(
    ip: &mut InstructionPointer<Idx, Space, Env>,
    space: &mut Space,
    env: &mut Env,
) -> InstructionResult
where
    Idx: MotionCmds<Space, Env> + SrcIO<Space>,
    Space: FungeSpace<Idx>,
    Space::Output: FungeValue,
    Env: InterpreterEnv,
{
    let mut sysinfo_cells = Vec::<Space::Output>::new();
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
    sysinfo_cells.push((size_of::<Space::Output>() as i32).into());

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
    sysinfo_cells.push(Idx::rank().into());

    // 8. IP ID
    sysinfo_cells.push(ip.id);

    // 9. IP team number
    sysinfo_cells.push(0.into());

    // 10. Position
    let mut tmp_vec = Vec::new();
    Idx::push_vector_onto(&mut tmp_vec, ip.location);
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    // 11. Delta
    let mut tmp_vec = Vec::new();
    Idx::push_vector_onto(&mut tmp_vec, ip.delta);
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    // 12. Storage offset
    let mut tmp_vec = Vec::new();
    Idx::push_vector_onto(&mut tmp_vec, ip.storage_offset);
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    // 13. Least point
    let mut tmp_vec = Vec::new();
    let least_idx = space.min_idx().unwrap_or(Idx::origin());
    Idx::push_vector_onto(&mut tmp_vec, least_idx);
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    // 14. Greatest point
    let mut tmp_vec = Vec::new();
    Idx::push_vector_onto(
        &mut tmp_vec,
        space.max_idx().unwrap_or(Idx::origin()) - least_idx,
    );
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    // 15 & 16: Time
    let timestamp = env.timestamp();
    let datetime = NaiveDateTime::from_timestamp(timestamp, 0);

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
    for stack in ip.stack_stack.iter().rev() {
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
