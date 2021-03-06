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

#![cfg(not(target_family = "wasm"))]

use std::cell::{RefCell, RefMut};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddrV4};
use std::os::raw::c_int;
use std::rc::Rc;

use hashbrown::HashMap;
use num::{FromPrimitive, ToPrimitive};
use socket2::{Domain, Protocol, Socket, Type};

use crate::interpreter::instruction_set::{sync_instruction, Instruction, InstructionResult};
use crate::interpreter::{Funge, MotionCmds};
use crate::InstructionPointer;

/// From the rcFunge docs:
///
/// "SOCK" 0x534F434B
///
/// A   (s -- prt addr s)   Accept a connection
/// B   (s ct prt addr -- ) Bind a socket
/// C   (s ct prt addr -- ) Open a connection
/// I   (0gnirts -- addr)   Convert an ascii ip address to a 32 bit address
/// K   (s -- )             Kill a connection
/// L   (n s -- )           Set a socket to listening mode (n=backlog size)
/// O   (n o s -- )         Set socket option
/// R   (V l s -- bytes)    Receive from a socket,
/// S   (pf typ pro -- s)   Create a socket
/// W   (V l s -- retcode)  Write to a socket
/// note: All functions act as r on failure
///
///  - addr:   32 bit destination address
///  - ct:
///     * 1=AF_UNIX
///     * 2=AF_INET
///  - o:
///     * 1=SO_DEBUG
///     * 2=SO_REUSEADDR
///     * 3=SO_KEEPALIVE
///     * 4=SO_DONTROUTE
///     * 5=SO_BROADCAST
///     * 6=OOBINLINE
///  - pf:
///     * 1=PF_UNIX
///     * 2=PF_INET
///  - prt:     Port to connect to
///  - s:       Socket identifier
///  - typ:
///     * 1=SOCK_DGRAM
///     * 2=SOCK_STREAM
///  - pro:
///     * 1=tcp
///     * 2=udp
///  - V:       Vector to io buffer
///
/// **Clarification**
///
/// The socket descriptor s used in these functions could be either an index
/// into a table of open sockets or else use the id returned by the OS. In
/// either case the socket identifier needs to be usable by other IPs,
/// therefore a socket table that is global to all IPs or else use the OS
/// descriptors.
///
/// ct=1 and pf=1 are a broken spec and should not be implemented. Usage of
/// either of these should reflect.
pub fn load<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    let mut layer = HashMap::<char, Instruction<F>>::new();
    layer.insert('A', sync_instruction(accept));
    layer.insert('B', sync_instruction(bind));
    layer.insert('C', sync_instruction(connect));
    layer.insert('I', sync_instruction(ipaddr));
    layer.insert('K', sync_instruction(kill));
    layer.insert('L', sync_instruction(listen));
    layer.insert('O', sync_instruction(setopt));
    layer.insert('R', sync_instruction(recv));
    layer.insert('S', sync_instruction(socket_create));
    layer.insert('W', sync_instruction(write));
    ip.instructions.add_layer(layer);
    true
}

pub fn unload<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> bool {
    ip.instructions
        .pop_layer(&"ABCIKLORSW".chars().collect::<Vec<char>>())
}

fn get_socketlist<F: Funge>(ip: &mut InstructionPointer<F>) -> RefMut<Vec<Option<Socket>>> {
    if !ip.private_data.contains_key("SOCK.sockets") {
        ip.private_data.insert(
            "SOCK.sockets".to_owned(),
            Rc::new(RefCell::new(Vec::<Option<Socket>>::new())),
        );
    }
    ip.private_data
        .get("SOCK.sockets")
        .and_then(|any_ref| any_ref.downcast_ref::<RefCell<Vec<Option<Socket>>>>())
        .map(|refcell| refcell.borrow_mut())
        .unwrap()
}

fn push_socket<F: Funge>(ip: &mut InstructionPointer<F>, socket: Socket) -> usize {
    let mut sock_idx = None;
    // scope to limit the lifetime of sl
    let mut sl = get_socketlist(ip);
    for (i, s) in sl.iter().enumerate() {
        if s.is_none() {
            sock_idx = Some(i);
            break;
        }
    }
    if let Some(i) = sock_idx {
        sl[i] = Some(socket);
        i
    } else {
        sl.push(Some(socket));
        sl.len() - 1
    }
}

fn socket_create<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    // get the parameters
    let proto = ip.pop();
    let typ = ip.pop();
    let pf = ip.pop();
    if pf != 2.into() {
        // only allow PF_INET
        ip.reflect();
        return InstructionResult::Continue;
    }

    let real_proto = match proto.to_i32().unwrap_or(-1) {
        1 => Some(Protocol::TCP),
        2 => Some(Protocol::UDP),
        0 => None,
        _ => {
            ip.reflect();
            return InstructionResult::Continue;
        }
    };

    if let Some(new_socket) = match typ.to_i32().unwrap_or_default() {
        1 => Socket::new(Domain::IPV4, Type::DGRAM, real_proto).ok(),
        2 => Socket::new(Domain::IPV4, Type::STREAM, real_proto).ok(),
        _ => None,
    } {
        let sock_idx = push_socket(ip, new_socket);
        ip.push(FromPrimitive::from_usize(sock_idx).unwrap());
    } else {
        ip.reflect();
    }

    InstructionResult::Continue
}

fn kill<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    // get the parameters
    let sock_id = if let Some(sock_id_usize) = ip.pop().to_usize() {
        sock_id_usize
    } else {
        ip.reflect();
        return InstructionResult::Continue;
    };

    let success = {
        let mut sl = get_socketlist(ip);
        if sock_id <= sl.len() {
            if let Some(sock) = &sl[sock_id] {
                sock.shutdown(Shutdown::Both).ok();
            }
            sl[sock_id] = None;
            true
        } else {
            false
        }
    };

    if !success {
        ip.reflect();
    }

    InstructionResult::Continue
}

fn setopt<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    // get the parameters
    let sock_id = if let Some(sock_id_usize) = ip.pop().to_usize() {
        sock_id_usize
    } else {
        ip.reflect();
        return InstructionResult::Continue;
    };
    let opt = ip.pop();
    let flag = ip.pop() != 0.into();

    let mut had_error = false;

    // Get the socket
    if let Some(sock) = get_socketlist(ip)
        .get(sock_id)
        .map(|o| o.as_ref())
        .unwrap_or_default()
    {
        if match opt.to_i32().unwrap_or_default() {
            // 1 => SO_DEBUG not supported
            2 => {
                // SO_REUSEADDR
                sock.set_reuse_address(flag).ok()
            }
            3 => {
                // SO_KEEPALIVE
                sock.set_keepalive(flag).ok()
            }
            // 4 => SO_DONTROUTE not supported
            5 => {
                // SO_BROADCAST
                sock.set_broadcast(flag).ok()
            }
            // 6 => OOBINLINE not supported
            // (though we could if we don't want Redox support)
            _ => None,
        }
        .is_none()
        {
            // some sort of error
            had_error = true;
        }
    } else {
        had_error = true;
    }

    if had_error {
        ip.reflect();
    }

    InstructionResult::Continue
}

fn bind<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    // get the parameters
    let addr = ip.pop().to_i32().unwrap_or_default();
    let port = if let Some(prt16) = ip.pop().to_u16() {
        prt16
    } else {
        ip.reflect();
        return InstructionResult::Continue;
    };
    let ct = ip.pop();
    let sock_id = if let Some(sock_id_usize) = ip.pop().to_usize() {
        sock_id_usize
    } else {
        ip.reflect();
        return InstructionResult::Continue;
    };

    if ct != 2.into() {
        // must be AF_INET
        ip.reflect();
        return InstructionResult::Continue;
    }

    let addr = SocketAddrV4::new((addr as u32).into(), port);

    let mut success = false;

    // Get the socket
    if let Some(sock) = get_socketlist(ip)
        .get(sock_id)
        .map(|o| o.as_ref())
        .unwrap_or_default()
    {
        success = sock.bind(&addr.into()).is_ok();
    }

    if !success {
        ip.reflect();
    }

    InstructionResult::Continue
}

fn connect<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    // get the parameters
    let addr = ip.pop().to_i32().unwrap_or_default();
    let port = if let Some(prt16) = ip.pop().to_u16() {
        prt16
    } else {
        ip.reflect();
        return InstructionResult::Continue;
    };
    let ct = ip.pop();
    let sock_id = if let Some(sock_id_usize) = ip.pop().to_usize() {
        sock_id_usize
    } else {
        ip.reflect();
        return InstructionResult::Continue;
    };

    if ct != 2.into() {
        // must be AF_INET
        ip.reflect();
        return InstructionResult::Continue;
    }

    let addr = SocketAddrV4::new((addr as u32).into(), port);

    let mut success = false;

    // Get the socket
    if let Some(sock) = get_socketlist(ip)
        .get(sock_id)
        .map(|o| o.as_ref())
        .unwrap_or_default()
    {
        success = sock.connect(&addr.into()).is_ok();
    }

    if !success {
        ip.reflect();
    }

    InstructionResult::Continue
}

fn listen<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    // get the parameters
    let sock_id = if let Some(sock_id_usize) = ip.pop().to_usize() {
        sock_id_usize
    } else {
        ip.reflect();
        return InstructionResult::Continue;
    };

    let backlog = ip.pop().to_i32().unwrap_or(1) as c_int;

    let mut success = false;

    // Get the socket
    if let Some(sock) = get_socketlist(ip)
        .get(sock_id)
        .map(|o| o.as_ref())
        .unwrap_or_default()
    {
        success = sock.listen(backlog).is_ok();
    }

    if !success {
        ip.reflect();
    }

    InstructionResult::Continue
}

fn accept<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    // get the parameters
    let sock_id = if let Some(sock_id_usize) = ip.pop().to_usize() {
        sock_id_usize
    } else {
        ip.reflect();
        return InstructionResult::Continue;
    };

    let mut success = false;

    let accept_result = get_socketlist(ip)
        .get(sock_id)
        .map(|o| o.as_ref())
        .unwrap_or_default()
        .and_then(|sock| sock.accept().ok());

    if let Some((client_sock, client_addr)) = accept_result {
        success = true;
        let v4_addr = client_addr.as_socket_ipv4().unwrap();
        ip.push((v4_addr.port() as i32).into());
        ip.push((u32::from(*v4_addr.ip()) as i32).into());
        // store the socket
        let sock_idx = push_socket(ip, client_sock);
        ip.push(FromPrimitive::from_usize(sock_idx).unwrap());
    }

    if !success {
        ip.reflect();
    }

    InstructionResult::Continue
}

fn recv<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    // get the parameters
    let sock_id = if let Some(sock_id_usize) = ip.pop().to_usize() {
        sock_id_usize
    } else {
        ip.reflect();
        return InstructionResult::Continue;
    };
    let max_count = ip.pop();
    let mut loc = MotionCmds::pop_vector(ip) + ip.storage_offset;
    let mut buf = vec![0_u8; max_count.to_usize().unwrap_or_default()];

    let read_result = get_socketlist(ip)
        .get_mut(sock_id)
        .map(|o| o.as_ref())
        .unwrap_or_default()
        .and_then(|mut sock| sock.read(&mut buf).ok());

    if let Some(count) = read_result {
        // copy data to fungespace
        for b in buf[0..count].iter() {
            space[loc] = (*b as i32).into();
            loc = loc.one_further();
        }
        ip.push(F::Value::from_usize(count).unwrap_or_else(|| 0.into()));
    } else {
        ip.reflect();
    }

    InstructionResult::Continue
}

fn write<F: Funge>(
    ip: &mut InstructionPointer<F>,
    space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    // get the parameters
    let sock_id = if let Some(sock_id_usize) = ip.pop().to_usize() {
        sock_id_usize
    } else {
        ip.reflect();
        return InstructionResult::Continue;
    };
    let count = ip.pop().to_usize().unwrap_or_default();
    let mut loc = MotionCmds::pop_vector(ip) + ip.storage_offset;
    let mut buf = vec![0_u8; count];
    for elem in buf.iter_mut().take(count) {
        *elem = (space[loc] & 0xff.into()).to_u8().unwrap_or_default();
        loc = loc.one_further();
    }

    let write_result = get_socketlist(ip)
        .get_mut(sock_id)
        .map(|o| o.as_ref())
        .unwrap_or_default()
        .and_then(|mut sock| sock.write_all(&buf).ok());

    if write_result.is_some() {
        ip.push(FromPrimitive::from_usize(buf.len()).unwrap_or_else(|| 0.into()));
    } else {
        ip.reflect();
    }

    InstructionResult::Continue
}

fn ipaddr<F: Funge>(
    ip: &mut InstructionPointer<F>,
    _space: &mut F::Space,
    _env: &mut F::Env,
) -> InstructionResult {
    let ip_string = ip.pop_0gnirts();

    if let Ok(addr) = ip_string.parse::<Ipv4Addr>() {
        let addr_long: u32 = addr.into();
        ip.push((addr_long as i32).into());
    } else {
        ip.reflect();
    }

    InstructionResult::Continue
}
