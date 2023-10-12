/* Copyright (c) 2023 Jeremy Davis (jeremydavis519@gmail.com)
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

#ifndef __PHOENIX_FCNTL_H
#define __PHOENIX_FCNTL_H

#include <stdio.h>
#include <sys/stat.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/* For fcntl()'s `cmd` argument */
#define F_DUPFD          1
#define F_DUPFD_CLOEXEC  2
#define F_GETFD          3
#define F_SETFD          4
#define F_GETFL          5
#define F_SETFL          6
#define F_GETLK          7
#define F_SETLK          8
#define F_SETLKW         9
#define F_GETOWN        10
#define F_SETOWN        11

/* For fcntl()'s file descriptor flags */
#define FD_CLOEXEC 1

/* For the `l_type` argument used for record locking with fcntl() */
#define F_RDLCK 1
#define F_UNLCK 2
#define F_WRLCK 3

/* For the `oflag` arguments of open() and openat() */
#define O_CLOEXEC   0x0100
#define O_CREAT     0x0200
#define O_DIRECTORY 0x0400
#define O_EXCL      0x0800
#define O_NOCTTY    0x1000
#define O_NOFOLLOW  0x2000
#define O_TRUNC     0x4000
#ifdef __GNUC__
#if __INT_MAX__ >= 0x8000
#define O_TTY_INIT  0x8000
#else
#define O_TTY_INIT  (-0x7fff & ~0x7fff) /* Equivalent to -0x8000 on 2's complement but also works with 1's complement and sign-magnitude */
#endif
#else
#error "fcntl.h currently requires GCC."
#endif

/* File status flags for open(), openat(), and fcntl() */
#define O_APPEND    0x0008
#define O_DSYNC     0x0010
#define O_NONBLOCK  0x0020
#define O_RSYNC     0x0040
#define O_SYNC      0x0080

/* File access modes for open(), openat(), and fcntl() */
#define O_ACCMODE   0x0007
#define O_EXEC      0x0001
#define O_RDONLY    0x0002
#define O_SEARCH    0x0003
#define O_WRONLY    0x0004
#define O_RDWR      0x0006

/* Special value for the `fd` argument of *at() functions */
#define AT_FDCWD -1

/* Flags for various *at() functions */
#define AT_EACCESS          0x01

#define AT_SYMLINK_NOFOLLOW 0x02

#define AT_SYMLINK_FOLLOW   0x04

#define AT_REMOVEDIR        0x08

/* For the `advice` argument of posix_fadvise() */
#define POSIX_FADV_NORMAL     1
#define POSIX_FADV_NOREUSE    2
#define POSIX_FADV_DONTNEED   3
#define POSIX_FADV_WILLNEED   4
#define POSIX_FADV_RANDOM     5
#define POSIX_FADV_SEQUENTIAL 6

struct flock {
    short l_type;
    short l_whence;
    off_t l_start;
    off_t l_len;
    pid_t l_pid;
};

int creat(const char*, mode_t);
int fcntl(int fildes, int cmd, ...);
int open(const char* path, int oflag, ...);
int openat(int fildes, const char* path, int oflag, ...);
int posix_fadvise(int fildes, off_t offset, off_t len, int advice);
int posix_fallocate(int fildes, off_t offset, off_t len);

#ifdef __cplusplus
}
#endif

#endif /* __PHOENIX_FCNTL_H */
