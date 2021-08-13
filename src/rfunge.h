/*
rfunge -- a Funge-98 interpreter
C Bindings - rfunge.h 
Copyright (C) 2021 Thomas Jollans

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

#ifndef RFUNGE_H_
#define RFUNGE_H_

#ifdef _WIN32
typedef long ssize_t;
#else
#include <sys/types.h>
#endif
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Typedefs for I/O callbacks */
typedef ssize_t (*rfunge_write_cb)(const char *, size_t, void*);
typedef ssize_t (*rfunge_read_cb)(char *, size_t, void*);

/* Opaque data type for the interpreter */
struct RFungeBFInterp_;
typedef struct RFungeBFInterp_ RFungeBFInterpreter;

extern RFungeBFInterpreter * rfunge_new_befunge_interpreter(
    bool unicode_mode,
    rfunge_write_cb out_cb,
    rfunge_read_cb in_cb,
    rfunge_write_cb err_cb,
    void *user_data);

extern void rfunge_free_interpreter(RFungeBFInterpreter *interp);
extern bool rfunge_load_src(RFungeBFInterpreter *interp, const char *buf, size_t len);
extern void rfunge_run(RFungeBFInterpreter *interp);


#ifdef __cplusplus
} // extern "C"
#endif


#endif /* RFUNGE_H_ */
