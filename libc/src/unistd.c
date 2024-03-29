/* Copyright (c) 2022-2024 Jeremy Davis (jeremydavis519@gmail.com)
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

/* Miscellaneous constants, types, and functions defined by POSIX
   https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/unistd.h.html */

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <stdnoreturn.h>
#include <unistd.h>
#include <phoenix.h>

#define FAIL(value) do { result = (value); goto fail; } while (0)
#define EFAIL(errnum) do { errno = (errnum); goto fail; } while (0)

typedef enum {
    FDT_NONE        = 0,
    FDT_PIPE_READER = 1,
    FDT_PIPE_WRITER = 2
} FDType;

typedef struct {
    _Atomic(FDType)      type; /* Must be FDT_PIPE_READER */
    _PHOENIX_PipeReader* reader;
    int                  file_descriptor_flags;
    int                  file_status_flags;
} FDPipeReader;

typedef struct {
    _Atomic(FDType)      type; /* Must be FDT_PIPE_WRITER */
    _PHOENIX_PipeWriter* writer;
    int                  file_descriptor_flags;
    int                  file_status_flags;
} FDPipeWriter;

typedef union {
    _Atomic(FDType) type; /* Doubles as a flag for allocating a file descriptor */
    FDPipeReader    pipe_reader;
    FDPipeWriter    pipe_writer;
} FileDescription;

static FileDescription file_descriptions[OPEN_MAX] = {0};

/* Allocates a file descriptor and returns its index. Return -1 on failure. */
static int allocate_file_descriptor(FDType type) {
    for (int i = 0; i < OPEN_MAX; ++i) {
        FDType none = FDT_NONE;
        if (atomic_compare_exchange_strong_explicit(&file_descriptions[i].type, &none, type, memory_order_acq_rel, memory_order_acquire)) {
            return i;
        }
    }
    return -1;
}

/* Frees the given file descriptor if it passes a bounds check. */
static void free_file_descriptor(int fildes) {
    if (fildes < 0 || fildes >= OPEN_MAX) return;
    atomic_store_explicit(&file_descriptions[fildes].type, FDT_NONE, memory_order_release);
}

ssize_t write_impl(int fildes, const void* buf, size_t nbyte, int use_o_append);

/* TODO
int          access(const char*, int);
unsigned int alarm(unsigned int);
int          chdir(const char*);
int          chown(const char*, uid_t, gid_t); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/close.html */
int close(int fildes) {
    if (fildes < 0 || fildes >= OPEN_MAX) EFAIL(EBADF);

    FileDescription* file_description = &file_descriptions[fildes];

    FDPipeReader* pr;
    FDPipeWriter* pw;

    /* FIXME: "If close() is interrupted by a signal that is to be caught, it shall return -1 with errno set to [EINTR]
     *        and the state of fildes is unspecified." (Can we just finish closing the file descriptor anyway?) */
    /* FIXME: "If an I/O error occurred while reading from or writing to the file system during close(), it may return -1
     *        with errno set to [EIO]; if this error is returned, the state of fildes is unspecified." */

    switch (atomic_exchange_explicit(&file_description->type, FDT_NONE, memory_order_acq_rel)) {
    case FDT_NONE:
        EFAIL(EBADF);

    case FDT_PIPE_READER:
        pr = &file_description->pipe_reader;
        _PHOENIX_pipe_free_reader(pr->reader);
        break;

    case FDT_PIPE_WRITER:
        pw = &file_description->pipe_writer;
        _PHOENIX_pipe_free_writer(pw->writer);
        break;

    default:
        /* Unrecognized file descriptor type. This is almost certainly a bug in libc. */
        EFAIL(EINTERNAL);
    }

    return 0;

fail:
    return -1;
}

/* TODO
size_t       confstr(int, char*, size_t);
char*        crypt(const char*, const char*);
int          dup(int);
int          dup2(int, int); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/_exit.html */
noreturn
void _exit(int status) {
    _Exit(status);
}

/* TODO
void         encrypt(char [64], int);
int          execl(const char*, const char*, ...);
int          execle(const char*, const char*, ...);
int          execlp(const char*, const char*, ...);
int          execv(const char*, char* const []);
int          execve(const char*, char* const [], char* const []);
int          execvp(const char*, char* const []);
int          faccessat(int, const char*, int, int);
int          fchdir(int);
int          fchown(int, uid_t, gid_t);
int          fchownat(int, const char*, uid_t, gid_t, int);
int          fdatasync(int);
int          fexecve(int, char* const [], char* const []);
pid_t        fork(void);
long         fpathconf(int, int);
int          fsync(int);
int          ftruncate(int, off_t);
char*        getcwd(char*, size_t);
gid_t        getegid(void);
uid_t        geteuid(void);
gid_t        getgid(void);
int          getgroups(int, gid_t []);
long         gethostid(void);
int          gethostname(char*, size_t);
char*        getlogin(void);
int          getlogin_r(char*, size_t);
int          getopt(int, char* const [], const char*);
pid_t        getpgid(pid_t);
pid_t        getpgrp(void);
pid_t        getpid(void);
pid_t        getppid(void);
pid_t        getsid(pid_t);
uid_t        getuid(void);
int          isatty(int);
int          lchown(const char*, uid_t, gid_t);
int          link(const char*, const char*);
int          linkat(int, const char*, int, const char*, int);
int          lockf(int, int, off_t); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/lseek.html */
off_t lseek(int fildes, off_t offset, int whence) {
    if (fildes < 0 || fildes >= OPEN_MAX) EFAIL(EBADF);

    FileDescription* file_description = &file_descriptions[fildes];

    switch (atomic_load_explicit(&file_description->type, memory_order_acquire)) {
    case FDT_NONE:
        EFAIL(EBADF);

    case FDT_PIPE_WRITER:
    case FDT_PIPE_READER:
        EFAIL(ESPIPE);

    default:
        /* Unrecognized file descriptor type. This is almost certainly a bug in libc. */
        EFAIL(EINTERNAL);
    }

fail:
    return -1;
}

/* TODO
int          nice(int);
long         pathconf(const char*, int);
int          pause(void); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/pipe.html */
int pipe(int fildes[2]) {
    int reader = -1, writer = -1;
    _PHOENIX_PipeReader* pipe_reader = NULL;
    _PHOENIX_PipeWriter* pipe_writer = NULL;

    if (_PHOENIX_pipe_new(&pipe_reader, &pipe_writer)) EFAIL(ENOMEM);

    if ((reader = allocate_file_descriptor(FDT_PIPE_READER)) < 0) EFAIL(EMFILE);
    if ((writer = allocate_file_descriptor(FDT_PIPE_WRITER)) < 0) EFAIL(EMFILE);

    file_descriptions[reader].pipe_reader.reader = pipe_reader;
    file_descriptions[reader].pipe_reader.file_descriptor_flags = 0;
    file_descriptions[reader].pipe_reader.file_status_flags = 0;

    file_descriptions[writer].pipe_writer.writer = pipe_writer;
    file_descriptions[writer].pipe_writer.file_descriptor_flags = 0;
    file_descriptions[writer].pipe_writer.file_status_flags = 0;

    /* FIXME: "The pipe's user ID shall be set to the effective user ID of the calling process." */
    /* FIXME: "The pipe's group ID shall be set to the effective group ID of the calling process." */

    fildes[0] = reader;
    fildes[1] = writer;

    return 0;

fail:
    free_file_descriptor(reader);
    free_file_descriptor(writer);
    _PHOENIX_pipe_free_reader(pipe_reader);
    _PHOENIX_pipe_free_writer(pipe_writer);
    return -1;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/pread.html */
ssize_t pread(int fildes, void* buf, size_t nbyte, off_t offset) {
    off_t orig_offset = lseek(fildes, 0, SEEK_CUR);
    if (orig_offset == -1) return -1;

    if (lseek(fildes, offset, SEEK_SET) == -1) return -1;
    ssize_t result = read(fildes, buf, nbyte);
    if (lseek(fildes, orig_offset, SEEK_SET) == -1) return -1;

    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/pwrite.html */
ssize_t pwrite(int fildes, const void* buf, size_t nbyte, off_t offset) {
    off_t orig_offset = lseek(fildes, 0, SEEK_CUR);
    if (orig_offset == -1) return -1;

    if (lseek(fildes, offset, SEEK_SET) == -1) return -1;
    ssize_t result = write_impl(fildes, buf, nbyte, 0);
    if (lseek(fildes, orig_offset, SEEK_SET) == -1) return -1;

    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/read.html */
ssize_t read(int fildes, void* buf, size_t nbyte) {
    if (fildes < 0 || fildes >= OPEN_MAX) EFAIL(EBADF);

    if (nbyte > SSIZE_MAX) nbyte = SSIZE_MAX;

    FileDescription* file_description = &file_descriptions[fildes];

    FDPipeReader* pr;

    /* FIXME: "If read() is interrupted by a signal before it reads any data, it shall return -1 with errno set to [EINTR]." */
    /* TODO: "If the O_DSYNC and O_RSYNC bits have been set, read I/O operations on the file descriptor shall complete as defined by synchronized
     *        I/O data integrity completion. If the O_SYNC and O_RSYNC bits have been set, read I/O operations on the file descriptor shall
     *        complete as defined by synchronized I/O file integrity completion." */

    ssize_t bytes_read = 0;

    switch (atomic_load_explicit(&file_description->type, memory_order_acquire)) {
    case FDT_NONE:
    case FDT_PIPE_WRITER:
        EFAIL(EBADF); /* "The fildes argument is not a valid file descriptor open for reading." */

    case FDT_PIPE_READER:
        pr = &file_description->pipe_reader;
        for (;;) {
            bytes_read = _PHOENIX_pipe_read(pr->reader, buf, nbyte);
            if (bytes_read == -1) {
                /* EOF: pipe has no writers. */
                bytes_read = 0;
                break;
            }
            if (bytes_read == 0) {
                /* Pipe has writers but is currently empty. */
                if (pr->file_status_flags & O_NONBLOCK) EFAIL(EAGAIN);
                _PHOENIX_thread_sleep(0); /* Wait for some data. */
            }
        };
        break;

    default:
        /* Unrecognized file descriptor type. This is almost certainly a bug in libc. */
        EFAIL(EINTERNAL);
    }

    /* TODO: "Upon successful completion, where nbyte is greater than 0, read() shall mark for update the last data access timestamp of the file" */

    return bytes_read;

fail:
    return -1;
}

/* TODO
ssize_t      readlink(const char* restrict, char* restrict, size_t);
ssize_t      readlinkat(int, const char* restrict, char* restrict, size_t);
int          rmdir(const char*);
int          setegid(gid_t);
int          seteuid(uid_t);
int          setgid(gid_t);
int          setpgid(pid_t, pid_t);
pid_t        setpgrp(void);
int          setregid(gid_t, gid_t);
int          setreuid(uid_t, uid_t);
pid_t        setsid(void);
int          setuid(uid_t); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/sleep.html */
unsigned int sleep(unsigned int seconds) {
    _PHOENIX_thread_sleep((uint64_t)seconds * 1000000000);
    /* FIXME: If this thread receives a signal that "invokes a signal-catching function or terminates the process", return early
     *        with the number of seconds left until the duration will have elapsed. */
    return 0;
}

/* TODO
void         swab(const void* restrict, void* restrict, ssize_t);
int          symlink(const char*, const char*);
int          symlinkat(const char*, int, const char*);
void         sync(void);
long         sysconf(int);
pid_t        tcgetpgrp(int);
int          tcsetpgrp(int, pid_t);
int          truncate(const char*, off_t);
char*        ttyname(int);
int          ttyname_r(int, char*, size_t);
int          unlink(const char*);
int          unlinkat(int, const char*, int);
*/

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/write.html */
ssize_t write(int fildes, const void* buf, size_t nbyte) {
    return write_impl(fildes, buf, nbyte, 1);
}

/* Implements the logic of `write` and `pwrite` in one place. */
ssize_t write_impl(int fildes, const void* buf, size_t nbyte, int use_o_append) {
    if (fildes < 0 || fildes >= OPEN_MAX) EFAIL(EBADF);

    if (nbyte > SSIZE_MAX) nbyte = SSIZE_MAX;

    FileDescription* file_description = &file_descriptions[fildes];

    FDPipeWriter* pw;

    /* TODO: "If write() is interrupted by a signal..." */
    /* TODO: "If the O_DSYNC bit has been set, write I/O operations on the file descriptor shall complete as defined by synchronized I/O data integrity completion." */
    /* TODO: "If the O_SYNC bit has been set, write I/O operations on the file descriptor shall complete as defined by synchronized I/O file integrity completion." */

    ssize_t bytes_written = 0;

    switch (atomic_load_explicit(&file_description->type, memory_order_acquire)) {
    case FDT_NONE:
    case FDT_PIPE_READER:
        EFAIL(EBADF);

    case FDT_PIPE_WRITER:
        pw = &file_description->pipe_writer;
        for (;;) {
            bytes_written = _PHOENIX_pipe_write(pw->writer, buf, nbyte);
            if (bytes_written == -1) {
                /* Pipe has no readers. */
                /* TODO: "A SIGPIPE signal shall also be sent to the thread." */
                EFAIL(EPIPE);
            }
            if (bytes_written == 0) {
                /* Pipe has readers but is currently full. */
                if (pw->file_status_flags & O_NONBLOCK) EFAIL(EAGAIN);
                _PHOENIX_thread_sleep(0); /* Wait for the pipe to clear. */
            }
        };
        break;

    default:
        /* Unrecognized file descriptor type. This is almost certainly a bug in libc. */
        EFAIL(EINTERNAL);
    }

    return bytes_written;

fail:
    return -1;
}
