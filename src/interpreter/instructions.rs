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
use std::mem::size_of;

use chrono::prelude::Utc;
use chrono::{Datelike, Timelike};
use num::ToPrimitive;
use pkg_version::{pkg_version_major, pkg_version_minor, pkg_version_patch};

use super::instruction_set::exec_instruction;
use super::motion::MotionCmds;
use super::{ExecMode, IOMode};
use super::{InstructionContext, InstructionResult, InterpreterEnv, Funge};
use crate::fungespace::{FungeSpace, FungeValue, SrcIO, FungeIndex};

pub async fn iterate<F: Funge>(mut ctx: InstructionContext<F>) -> (InstructionContext<F>, InstructionResult)
{
    let n = ctx.ip.pop();
    let (mut new_loc, new_val_ref) = ctx.space.move_by(ctx.ip.location, ctx.ip.delta);
    let mut new_val = *new_val_ref;
    let mut loop_result = InstructionResult::Continue;
    let mut new_val_c = new_val.to_char();
    while new_val_c == ';' {
        // skip what must be skipped
        // fake-execute!
        let old_loc = ctx.ip.location;
        ctx.ip.location = new_loc;
        let (ctx_, _) = exec_instruction(new_val, ctx).await;
        ctx = ctx_;
        let (new_loc2, new_val_ref) = ctx.space.move_by(ctx.ip.location, ctx.ip.delta);
        new_loc = new_loc2;
        new_val = *new_val_ref;
        ctx.ip.location = old_loc;
        new_val_c = new_val.to_char();
    }
    if let Some(n) = n.to_usize() {
        if n == 0 {
            // surprising behaviour! 1k leads to the next instruction
            // being executed twice, 0k to it being skipped
            ctx.ip.location = new_loc;
            loop_result = InstructionResult::Continue;
        } else {
            let mut forks = 0;
            for _ in 0..n {
                let (ctx_, result) = exec_instruction(new_val, ctx).await;
                ctx = ctx_;
                match result {
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
        ctx.ip.reflect();
    }
    (ctx, loop_result)
}

pub fn begin_block<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    if let Some(n) = ctx.ip.pop().to_isize() {
        // take n items off the SOSS (old TOSS)
        let n_to_take = max(0, min(n, ctx.ip.stack().len() as isize));
        let zeros_for_toss = max(0, n - n_to_take);
        let zeros_for_soss = max(0, -n);

        let split_idx = ctx.ip.stack().len() - n_to_take as usize;
        let mut transfer_elems = ctx.ip.stack_mut().split_off(split_idx);

        for _ in 0..zeros_for_soss {
            ctx.ip.push(0.into());
        }

        let offset = ctx.ip.storage_offset;
        MotionCmds::push_vector(&mut ctx.ip, offset); // onto SOSS / old TOSS

        // create a new stack
        ctx.ip.stack_stack.insert(0, Vec::new());

        for _ in 0..zeros_for_toss {
            ctx.ip.push(0.into());
        }

        ctx.ip.stack_mut().append(&mut transfer_elems);

        ctx.ip.storage_offset = ctx.ip.location + ctx.ip.delta;
    } else {
        ctx.ip.reflect();
    }

    InstructionResult::Continue
}

pub fn end_block<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    if ctx.ip.stack_stack.len() > 1 {
        if let Some(n) = ctx.ip.pop().to_isize() {
            let mut toss = ctx.ip.stack_stack.remove(0);

            // restore the storage offset
            ctx.ip.storage_offset = MotionCmds::pop_vector(&mut ctx.ip);

            let n_to_take = max(0, min(n, toss.len() as isize));
            let zeros_for_soss = max(0, n - n_to_take);
            let n_to_pop = max(0, -n);

            if n_to_pop > 0 {
                for _ in 0..n_to_pop {
                    ctx.ip.pop();
                }
            } else {
                for _ in 0..zeros_for_soss {
                    ctx.ip.push(0.into());
                }

                let split_idx = toss.len() - n_to_take as usize;
                ctx.ip.stack_mut().append(&mut toss.split_off(split_idx));
            }
        } else {
            ctx.ip.reflect();
        }
    } else {
        ctx.ip.reflect();
    }

    InstructionResult::Continue
}

pub fn stack_under_stack<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let nstacks = ctx.ip.stack_stack.len();
    if nstacks > 1 {
        if let Some(n) = ctx.ip.pop().to_isize() {
            match n.cmp(&0) {
                Ordering::Greater => {
                    for _ in 0..n {
                        let v = ctx.ip.stack_stack[1].pop().unwrap_or_else(|| 0.into());
                        ctx.ip.push(v);
                    }
                }
                Ordering::Less => {
                    for _ in 0..(-n) {
                        let v = ctx.ip.pop();
                        ctx.ip.stack_stack[1].push(v);
                    }
                }
                Ordering::Equal => {}
            }
        } else {
            ctx.ip.reflect();
        }
    } else {
        ctx.ip.reflect();
    }
    InstructionResult::Continue
}

pub fn input_file<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let filename = ctx.ip.pop_0gnirts();
    let flags = ctx.ip.pop();
    let dest = MotionCmds::pop_vector(&mut ctx.ip);

    match ctx.env.get_iomode() {
        IOMode::Binary => {
            if let Ok(src) = ctx.env.read_file(&filename) {
                if flags & 1.into() == 1.into() {
                    // "binary mode" = linear mode
                    let mut dest = dest;
                    for b in src {
                        ctx.space[dest] = (b as i32).into();
                        dest = dest.one_further();
                    }
                } else {
                    // "text mode"
                    let size = F::Idx::read_bin_at(&mut ctx.space, &dest, &src);
                    MotionCmds::push_vector(&mut ctx.ip, size);
                    MotionCmds::push_vector(&mut ctx.ip, dest);
                }
            } else {
                ctx.ip.reflect();
            }
        }
        IOMode::Text => {
            if let Some(src) = ctx
                .env
                .read_file(&filename)
                .ok()
                .and_then(|v| String::from_utf8(v).ok())
            {
                if flags & 1.into() == 1.into() {
                    // "binary mode" = linear mode
                    let mut dest = dest;
                    for c in src.chars() {
                        ctx.space[dest] = (c as i32).into();
                        dest = dest.one_further();
                    }
                } else {
                    // "text mode"
                    let size = F::Idx::read_str_at(&mut ctx.space, &dest, &src);
                    MotionCmds::push_vector(&mut ctx.ip, size);
                    MotionCmds::push_vector(&mut ctx.ip, dest);
                }
            } else {
                ctx.ip.reflect();
            }
        }
    }

    InstructionResult::Continue
}

pub fn output_file<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let filename = ctx.ip.pop_0gnirts();
    let flags = ctx.ip.pop();
    let start = MotionCmds::pop_vector(&mut ctx.ip);
    let size = MotionCmds::pop_vector(&mut ctx.ip);

    let strip = (flags & 1.into()) == 1.into();

    if match ctx.env.get_iomode() {
        IOMode::Binary => ctx.env.write_file(
            &filename,
            &F::Idx::get_src_bin(&ctx.space, &start, &size, strip),
        ),
        IOMode::Text => ctx.env.write_file(
            &filename,
            F::Idx::get_src_str(&ctx.space, &start, &size, strip).as_bytes(),
        ),
    }
    .is_err()
    {
        ctx.ip.reflect();
    }

    InstructionResult::Continue
}

pub fn execute<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    if ctx.env.have_execute() == ExecMode::Disabled {
        ctx.ip.reflect();
    } else {
        let cmd = ctx.ip.pop_0gnirts();
        ctx.ip.push(ctx.env.execute_command(&cmd).into());
    }

    InstructionResult::Continue
}

pub fn sysinfo<F: Funge>(ctx: &mut InstructionContext<F>) -> InstructionResult
{
    let mut sysinfo_cells = Vec::<F::Value>::new();
    // what should we push?
    let n = ctx.ip.pop();
    let exec_flag = ctx.env.have_execute();
    // Set everything up first

    // 1. flags
    let mut impl_flags = 0x1; // concurrent funge-98
    if ctx.env.have_file_input() {
        impl_flags |= 0x2
    }
    if ctx.env.have_file_output() {
        impl_flags |= 0x4
    }
    if exec_flag != ExecMode::Disabled {
        impl_flags |= 0x8
    }
    if !ctx.env.is_io_buffered() {
        impl_flags |= 0x10
    }
    sysinfo_cells.push(impl_flags.into());

    // 2. size of cell
    sysinfo_cells.push((size_of::<F::Value>() as i32).into());

    // 3. handprint
    sysinfo_cells.push(ctx.env.handprint().into());

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
    sysinfo_cells.push(F::Idx::rank().into());

    // 8. IP ID
    sysinfo_cells.push(ctx.ip.id);

    // 9. IP team number
    sysinfo_cells.push(0.into());

    // 10. Position
    let mut tmp_vec = Vec::new();
    F::Idx::push_vector_onto(&mut tmp_vec, ctx.ip.location);
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    // 11. Delta
    let mut tmp_vec = Vec::new();
    F::Idx::push_vector_onto(&mut tmp_vec, ctx.ip.delta);
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    // 12. Storage offset
    let mut tmp_vec = Vec::new();
    F::Idx::push_vector_onto(&mut tmp_vec, ctx.ip.storage_offset);
    sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

    let idx: F::Value = (sysinfo_cells.len() as i32).into();
    // Only calculate the next bit if we need it as it's quite expensive
    if n <= 0.into() || (n > idx && n <= idx + (2 * F::Idx::rank()).into()) {
        // 13. Least point

        let mut tmp_vec = Vec::new();
        let least_idx = ctx.space.min_idx().unwrap_or_else(F::Idx::origin);
        F::Idx::push_vector_onto(&mut tmp_vec, least_idx);
        sysinfo_cells.append(&mut tmp_vec.into_iter().rev().collect());

        // 14. Greatest point

        let mut tmp_vec = Vec::new();
        F::Idx::push_vector_onto(
            &mut tmp_vec,
            ctx.space.max_idx().unwrap_or_else(F::Idx::origin) - least_idx,
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
    sysinfo_cells.push((ctx.ip.stack_stack.len() as i32).into());

    // 18. sizes of stacks
    for stack in ctx.ip.stack_stack.iter() {
        sysinfo_cells.push((stack.len() as i32).into());
    }

    // 19. command line args
    for arg in ctx.env.argv().into_iter() {
        for c in arg.chars() {
            sysinfo_cells.push((c as i32).into());
        }
        sysinfo_cells.push(0.into());
    }
    sysinfo_cells.push(0.into());
    sysinfo_cells.push(0.into());

    // 20. environment
    for (key, value) in ctx.env.env_vars().into_iter() {
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
        let idx = ctx.ip.stack().len() as isize - pick_n.to_isize().unwrap();
        if idx >= 0 {
            ctx.ip.push(ctx.ip.stack()[idx as usize]);
        }
    } else if n > 0.into() {
        // pick one cell from sysinfo
        ctx.ip.push(sysinfo_cells[n.to_usize().unwrap() - 1]);
    } else {
        // push it all
        for cell in sysinfo_cells.into_iter().rev() {
            ctx.ip.push(cell);
        }
    }

    InstructionResult::Continue
}
