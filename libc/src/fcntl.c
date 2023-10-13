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

#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>

static int vopenat(int fildes, const char* path, int oflag, va_list args);

/* TODO
int creat(const char*, mode_t);
int fcntl(int fildes, int cmd, ...); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/open.html */
int open(const char* path, int oflag, ...) {
    va_list args;
    va_start(args, oflag);
    int result = vopenat(AT_FDCWD, path, oflag, args);
    va_end(args);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/openat.html */
int openat(int fildes, const char* path, int oflag, ...) {
    va_list args;
    va_start(args, oflag);
    int result = vopenat(fildes, path, oflag, args);
    va_end(args);
    return result;
}

int vopenat(int fildes, const char* path, int oflag, va_list args) {
    /* TODO */
    errno = ENFILE;
    return -1;
}

/* TODO
int posix_fadvise(int fildes, off_t offset, off_t len, int advice);
int posix_fallocate(int fildes, off_t offset, off_t len); */
