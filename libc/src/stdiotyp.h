/* Copyright (c) 2024 Jeremy Davis (jeremydavis519@gmail.com)
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software
 * and associated documentation files (the "Software"), to deal in the Software without restriction,
 * including without limitation the rights to use, copy, modify, merge, publish, distribute,
 * sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or
 * substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
 * NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
 * DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

#ifndef __PHOENIX_STDIOTYP_H
#define __PHOENIX_STDIOTYP_H

#include <stdatomic.h>
#include <sys/types.h>

typedef unsigned int CharWidth;
#define CW_UNSET    0
#define CW_NARROW   1
#define CW_WIDE     2

typedef unsigned int BufferMode;
/* The variants are _IOFBF, _IOLBF, and _IONBF. */

typedef unsigned int IOMode;
#define IO_READ    1
#define IO_WRITE   2
#define IO_RW      IO_READ | IO_WRITE

struct mbstate_t {
    /* TODO */
};

/* FIXME: This has to be defined in stdio.h, or else it'll be an incomplete type in client code. */
struct fpos_t {
    off_t     offset;         /* Number of bytes into the file */
    mbstate_t mb_parse_state; /* State of the multibyte character parser */
};

struct FILE {
    int            is_open         : 1;
    CharWidth      char_width      : 2;
    BufferMode     buffer_mode     : 2;
    IOMode         io_mode         : 2;
    int            eof             : 1;
    int            error           : 1;
    int            malloced_buffer : 1;
    const char*    path;
    int            fildes;       /* File descriptor */
    fpos_t         position;
    off_t          length;
    unsigned char* buffer;       /* Pointer to buffer being used, or NULL */
    size_t         buffer_size;
    size_t         buffer_index; /* Index of next byte to set in the buffer */
    atomic_size_t  lock_count;
    pthread_t      lock_owner;
    union {
        wint_t        wc;
        unsigned char c[sizeof(wint_t)];
    }              pushback_buffer;
    uint8_t        pushback_index;
};

#endif /* __PHOENIX_STDIOTYP_H */
