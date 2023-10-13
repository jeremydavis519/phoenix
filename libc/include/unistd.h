/* Copyright (c) 2022 Jeremy Davis (jeremydavis519@gmail.com)
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

#ifndef __PHOENIX_UNISTD_H
#define __PHOENIX_UNISTD_H

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdnoreturn.h>
#include <sys/types.h>

/* Version test macros */
#define _POSIX_VERSION  200809L
#define _POSIX2_VERSION 200809L
#define _XOPEN_VERSION  700

/* Constants for options and option groups */
/* -1: Option is not supported
    0: Option is accepted at compile-time but may not be supported at run-time
    Greater than 0: Option is guaranteed to be supported */
/* TODO: Support more of these options over time. */
#define _POSIX_ADVISORY_INFO                -1
#define _POSIX_ASYNCHRONOUS_IO              200809L
#define _POSIX_BARRIERS                     200809L
#define _POSIX_CHOWN_RESTRICTED             1
#define _POSIX_CLOCK_SELECTION              200809L
#define _POSIX_CPUTIME                      -1
#define _POSIX_FSYNC                        -1
#define _POSIX_IPV6                         -1
#define _POSIX_JOB_CONTROL                  1
#define _POSIX_MAPPED_FILES                 1
#define _POSIX_MEMLOCK                      -1
#define _POSIX_MEMLOCK_RANGE                -1
#define _POSIX_MEMORY_PROTECTION            200809L
#define _POSIX_MESSAGE_PASSING              -1
#define _POSIX_MONOTONIC_CLOCK              -1
#define _POSIX_NO_TRUNC                     1
#define _POSIX_PRIORITIZED_IO               -1
#define _POSIX_PRIORITY_SCHEDULING          -1
#define _POSIX_RAW_SOCKETS                  -1
#define _POSIX_READER_WRITER_LOCKS          200809L
#define _POSIX_REALTIME_SIGNALS             200809L
#define _POSIX_REGEXP                       1
#define _POSIX_SAVED_IDS                    1
#define _POSIX_SEMAPHORES                   1
#define _POSIX_SHARED_MEMORY_OBJECTS        -1
#define _POSIX_SHELL                        1
#define _POSIX_SPAWN                        -1
#define _POSIX_SPIN_LOCKS                   200809L
#define _POSIX_SPORADIC_SERVER              -1
#define _POSIX_SYNCHRONIZED_IO              -1
#define _POSIX_THREAD_ATTR_STACKADDR        -1
#define _POSIX_THREAD_ATTR_STACKSIZE        -1
#define _POSIX_THREAD_CPUTIME               -1
#define _POSIX_THREAD_PRIO_INHERIT          -1
#define _POSIX_THREAD_PRIO_PROTECT          -1
#define _POSIX_THREAD_PRIORITY_SCHEDULING   -1
#define _POSIX_THREAD_PROCESS_SHARED        -1
#define _POSIX_THREAD_ROBUST_PRIO_INHERIT   -1
#define _POSIX_THREAD_ROBUST_PRIO_PROTECT   -1
#define _POSIX_THREAD_SAFE_FUNCTIONS        200809L
#define _POSIX_THREAD_SPORADIC_SERVER       -1
#define _POSIX_THREADS                      200809L
#define _POSIX_TIMEOUTS                     200809L
#define _POSIX_TIMERS                       200809L
#define _POSIX_TRACE                        -1
#define _POSIX_TRACE_EVENT_FILTER           -1
#define _POSIX_TRACE_INHERIT                -1
#define _POSIX_TRACE_LOG                    -1
#define _POSIX_TYPED_MEMORY_OBJECTS         -1

#define _POSIX_V7_ILP32_OFF32               1
#define _POSIX_V7_ILP32_OFFBIG              1
#define _POSIX_V7_LP64_OFF64                1
#define _POSIX_V7_LPBIG_OFFBIG              1
#define _POSIX_V6_ILP32_OFF32               _POSIX_V7_ILP32_OFF32
#define _POSIX_V6_ILP32_OFFBIG              _POSIX_V7_ILP32_OFFBIG
#define _POSIX_V6_LP64_OFF64                _POSIX_V7_LP64_OFF64
#define _POSIX_V6_LPBIG_OFFBIG              _POSIX_V7_LPBIG_OFFBIG

#define _POSIX2_C_BIND                      200809L
#define _POSIX2_C_DEV                       -1
#define _POSIX2_CHAR_TERM                   -1
#define _POSIX2_FORT_DEV                    -1
#define _POSIX2_FORT_RUN                    -1
#define _POSIX2_LOCALEDEF                   -1
#define _POSIX2_PBS                         -1
#define _POSIX2_PBS_ACCOUNTING              -1
#define _POSIX2_PBS_CHECKPOINT              -1
#define _POSIX2_PBS_LOCATE                  -1
#define _POSIX2_PBS_MESSAGE                 -1
#define _POSIX2_PBS_TRACK                   -1
#define _POSIX2_SW_DEV                      -1
#define _POSIX2_UPE                         -1
#define _XOPEN_CRYPT                        -1
#define _XOPEN_ENH_I18N                     1
#define _XOPEN_REALTIME                     -1
#define _XOPEN_REALTIME_THREADS             -1
#define _XOPEN_SHM                          1
#define _XOPEN_STREAMS                      -1
#define _XOPEN_UNIX                         -1
#define _XOPEN_UUCP                         -1

/* Evaluation-time symbolic constants */
/* -1: Option is not supported on any file
    Anything else: Option is supported on every file
    Undefined: Option may be supported on some files and unsupported on others */
#define _POSIX_ASYNC_IO                     -1
#define _POSIX_PRIO_IO                      -1
#define _POSIX_SYNC_IO                      -1

/* Defined: Option applies to every file and every path in every file system
   Undefined: Option may or may not apply to any given file or path */
#undef _POSIX_TIMESTAMP_RESOLUTION
#undef _POSIX2_SYMLINKS

/* Constants for functions */
/* access() */
#define F_OK 0
#define R_OK 1
#define W_OK 2
#define X_OK 4

/* confstr() */
#define _CS_PATH                             0
#define _CS_POSIX_V7_ILP32_OFF32_CFLAGS      1
#define _CS_POSIX_V7_ILP32_OFF32_LDFLAGS     2
#define _CS_POSIX_V7_ILP32_OFF32_LIBS        3
#define _CS_POSIX_V7_ILP32_OFFBIG_CFLAGS     4
#define _CS_POSIX_V7_ILP32_OFFBIG_LDFLAGS    5
#define _CS_POSIX_V7_ILP32_OFFBIG_LIBS       6
#define _CS_POSIX_V7_LP64_OFF64_CFLAGS       7
#define _CS_POSIX_V7_LP64_OFF64_LDFLAGS      8
#define _CS_POSIX_V7_LP64_OFF64_LIBS         9
#define _CS_POSIX_V7_LPBIG_OFFBIG_CFLAGS    10
#define _CS_POSIX_V7_LPBIG_OFFBIG_LDFLAGS   11
#define _CS_POSIX_V7_LPBIG_OFFBIG_LIBS      12
#define _CS_POSIX_V7_THREADS_CFLAGS         13
#define _CS_POSIX_V7_THREADS_LDFLAGS        14
#define _CS_POSIX_V7_WIDTH_RESTRICTED_ENVS  15
#define _CS_V7_ENV                          16

/* lockf() */
#define F_LOCK      0
#define F_TEST      1
#define F_TLOCK     2
#define F_UNLOCK    3

/* pathconf() */
#define _PC_2_SYMLINKS                       0
#define _PC_ALLOC_SIZE_MIN                   1
#define _PC_ASYNC_IO                         2
#define _PC_CHOWN_RESTRICTED                 3
#define _PC_FILESIZEBITS                     4
#define _PC_LINK_MAX                         5
#define _PC_MAX_CANON                        6
#define _PC_MAX_INPUT                        7
#define _PC_NAME_MAX                         8
#define _PC_NO_TRUNC                         9
#define _PC_PATH_MAX                        10
#define _PC_PIPE_BUF                        11
#define _PC_PRIO_IO                         12
#define _PC_REC_INCR_XFER_SIZE              13
#define _PC_REC_MAX_XFER_SIZE               14
#define _PC_REC_MIN_XFER_SIZE               15
#define _PC_REC_XFER_ALIGN                  16
#define _PC_SYMLINK_MAX                     17
#define _PC_SYNC_IO                         18
#define _PC_TIMESTAMP_RESOLUTION            19
#define _PC_VDISABLE                        20

/* sysconf() */
#define _SC_2_C_BIND                          0
#define _SC_2_C_DEV                           1
#define _SC_2_CHAR_TERM                       2
#define _SC_2_FORT_DEV                        3
#define _SC_2_FORT_RUN                        4
#define _SC_2_LOCALEDEF                       5
#define _SC_2_PBS                             6
#define _SC_2_PBS_ACCOUNTING                  7
#define _SC_2_PBS_CHECKPOINT                  8
#define _SC_2_PBS_LOCATE                      9
#define _SC_2_PBS_MESSAGE                    10
#define _SC_2_PBS_TRACK                      11
#define _SC_2_SW_DEV                         12
#define _SC_2_UPE                            13
#define _SC_2_VERSION                        14
#define _SC_ADVISORY_INFO                    15
#define _SC_AIO_LISTIO_MAX                   16
#define _SC_AIO_MAX                          17
#define _SC_AIO_PRIO_DELTA_MAX               18
#define _SC_ARG_MAX                          19
#define _SC_ASYNCHRONOUS_IO                  20
#define _SC_ATEXIT_MAX                       21
#define _SC_BARRIERS                         22
#define _SC_BC_BASE_MAX                      23
#define _SC_BC_DIM_MAX                       24
#define _SC_BC_SCALE_MAX                     25
#define _SC_BC_STRING_MAX                    26
#define _SC_CHILD_MAX                        27
#define _SC_CLK_TCK                          28
#define _SC_CLOCK_SELECTION                  29
#define _SC_COLL_WEIGHTS_MAX                 30
#define _SC_CPUTIME                          31
#define _SC_DELAYTIMER_MAX                   32
#define _SC_EXPR_NEST_MAX                    33
#define _SC_FSYNC                            34
#define _SC_GETGR_R_SIZE_MAX                 35
#define _SC_GETPW_R_SIZE_MAX                 36
#define _SC_HOST_NAME_MAX                    37
#define _SC_IOV_MAX                          38
#define _SC_IPV6                             39
#define _SC_JOB_CONTROL                      40
#define _SC_LINE_MAX                         41
#define _SC_LOGIN_NAME_MAX                   42
#define _SC_MAPPED_FILES                     43
#define _SC_MEMLOCK                          44
#define _SC_MEMLOCK_RANGE                    45
#define _SC_MEMORY_PROTECTION                46
#define _SC_MESSAGE_PASSING                  47
#define _SC_MONOTONIC_CLOCK                  48
#define _SC_MQ_OPEN_MAX                      49
#define _SC_MQ_PRIO_MAX                      50
#define _SC_NGROUPS_MAX                      51
#define _SC_OPEN_MAX                         52
#define _SC_PAGE_SIZE                        53
#define _SC_PAGESIZE                         _SC_PAGE_SIZE
#define _SC_PRIORITIZED_IO                   55
#define _SC_PRIORITY_SCHEDULING              56
#define _SC_RAW_SOCKETS                      57
#define _SC_RE_DUP_MAX                       58
#define _SC_READER_WRITER_LOCKS              59
#define _SC_REALTIME_SIGNALS                 60
#define _SC_REGEXP                           61
#define _SC_RTSIG_MAX                        62
#define _SC_SAVED_IDS                        63
#define _SC_SEM_NSEMS_MAX                    64
#define _SC_SEM_VALUE_MAX                    65
#define _SC_SEMAPHORES                       66
#define _SC_SHARED_MEMORY_OBJECTS            67
#define _SC_SHELL                            68
#define _SC_SIGQUEUE_MAX                     69
#define _SC_SPAWN                            70
#define _SC_SPIN_LOCKS                       71
#define _SC_SPORADIC_SERVER                  72
#define _SC_SS_REPL_MAX                      73
#define _SC_STREAM_MAX                       74
#define _SC_SYMLOOP_MAX                      75
#define _SC_SYNCHRONIZED_IO                  76
#define _SC_THREAD_ATTR_STACKADDR            77
#define _SC_THREAD_ATTR_STACKSIZE            78
#define _SC_THREAD_CPUTIME                   79
#define _SC_THREAD_DESTRUCTOR_ITERATIONS     80
#define _SC_THREAD_KEYS_MAX                  81
#define _SC_THREAD_PRIO_INHERIT              82
#define _SC_THREAD_PRIO_PROTECT              83
#define _SC_THREAD_PRIORITY_SCHEDULING       84
#define _SC_THREAD_PROCESS_SHARED            85
#define _SC_THREAD_ROBUST_PRIO_INHERIT       86
#define _SC_THREAD_ROBUST_PRIO_PROTECT       87
#define _SC_THREAD_SAFE_FUNCTIONS            88
#define _SC_THREAD_SPORADIC_SERVER           89
#define _SC_THREAD_STACK_MIN                 90
#define _SC_THREAD_THREADS_MAX               91
#define _SC_THREADS                          92
#define _SC_TIMEOUTS                         93
#define _SC_TIMER_MAX                        94
#define _SC_TIMERS                           95
#define _SC_TRACE                            96
#define _SC_TRACE_EVENT_FILTER               97
#define _SC_TRACE_EVENT_NAME_MAX             98
#define _SC_TRACE_INHERIT                    99
#define _SC_TRACE_LOG                       100
#define _SC_TRACE_NAME_MAX                  101
#define _SC_TRACE_SYS_MAX                   102
#define _SC_TRACE_USER_EVENT_MAX            103
#define _SC_TTY_NAME_MAX                    104
#define _SC_TYPED_MEMORY_OBJECTS            105
#define _SC_TZNAME_MAX                      106
#define _SC_V7_ILP32_OFF32                  107
#define _SC_V7_ILP32_OFFBIG                 108
#define _SC_V7_LP64_OFF64                   109
#define _SC_V7_LPBIG_OFFBIG                 110
#define _SC_V6_ILP32_OFF32                  111
#define _SC_V6_ILP32_OFFBIG                 112
#define _SC_V6_LP64_OFF64                   113
#define _SC_V6_LPBIG_OFFBIG                 114
#define _SC_VERSION                         115
#define _SC_XOPEN_CRYPT                     116
#define _SC_XOPEN_ENH_I18N                  117
#define _SC_XOPEN_REALTIME                  118
#define _SC_XOPEN_REALTIME_THREADS          119
#define _SC_XOPEN_SHM                       120
#define _SC_XOPEN_STREAMS                   121
#define _SC_XOPEN_UNIX                      122
#define _SC_XOPEN_UUCP                      123
#define _SC_XOPEN_VERSION                   124

/* File streams */
#define STDIN_FILENO    0
#define STDOUT_FILENO   1
#define STDERR_FILENO   2

/* Terminal special character handling */
#define _POSIX_VDISABLE '\0'

#ifdef __cplusplus
extern "C" {
#endif /* __cplusplus */

#if !defined(__cplusplus) && __STDC_VERSION__ >= 199901L
/* Use a prefix allowed by POSIX. */
#define SEEK_restrict restrict
#else
#define SEEK_restrict
#endif /* __cplusplus and __STDC_VERSION__ */

/* Declarations */
int          access(const char*, int);
unsigned int alarm(unsigned int);
int          chdir(const char*);
int          chown(const char*, uid_t, gid_t);
int          close(int);
size_t       confstr(int, char*, size_t);
char*        crypt(const char*, const char*);
int          dup(int);
int          dup2(int, int);
noreturn
void         _exit(int status);
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
int          lockf(int, int, off_t);
off_t        lseek(int fildes, off_t offset, int whence);
int          nice(int);
long         pathconf(const char*, int);
int          pause(void);
int          pipe(int fildes[2]);
ssize_t      pread(int fildes, void* buf, size_t nbyte, off_t offset);
ssize_t      pwrite(int, const void*, size_t, off_t);
ssize_t      read(int fildes, void* buf, size_t nbyte);
ssize_t      readlink(const char* SEEK_restrict, char* SEEK_restrict, size_t);
ssize_t      readlinkat(int, const char* SEEK_restrict, char* SEEK_restrict, size_t);
int          rmdir(const char*);
int          setegid(gid_t);
int          seteuid(uid_t);
int          setgid(gid_t);
int          setpgid(pid_t, pid_t);
pid_t        setpgrp(void);
int          setregid(gid_t, gid_t);
int          setreuid(uid_t, uid_t);
pid_t        setsid(void);
int          setuid(uid_t);
unsigned int sleep(unsigned int);
void         swab(const void* SEEK_restrict, void* SEEK_restrict, ssize_t);
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
ssize_t      write(int, const void*, size_t);

extern char*  optarg;
extern int    opterr, optind, optopt;

#ifdef __cplusplus
}
#endif /* __cplusplus */

#endif /* __PHOENIX_UNISTD_H */
