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

/* Types defined by the POSIX specification
   (https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/sys_types.h.html) */

#ifndef __PHOENIX_SYS_TYPES_H
#define __PHOENIX_SYS_TYPES_H

#include <stddef.h>
#include <stdint.h>

typedef int64_t blkcnt_t;
typedef int64_t blksize_t;
/* TODO
typedef (unsigned) fsblkcnt_t;
typedef (unsigned) fsfilcnt_t; */
typedef uint64_t dev_t; /* TODO: What's the format of a dev_t value? MAC address + 2-byte local device number? */
typedef uint64_t ino_t;
/* TODO
typedef  key_t; */
typedef uint16_t mode_t;
typedef uint64_t nlink_t;

typedef int64_t off_t;
#define OFF_MIN INT64_MIN
#define OFF_MAX INT64_MAX

typedef int64_t ssize_t;

typedef uint8_t clockid_t;
typedef uint64_t clock_t;
typedef int32_t suseconds_t;
typedef uint64_t time_t;
typedef uint16_t timer_t;

typedef uint32_t id_t;
typedef uint32_t gid_t;
typedef uint32_t uid_t;
typedef int64_t pid_t;

typedef struct pthread_attr_t pthread_attr_t;
typedef struct pthread_barrier_t pthread_barrier_t;
typedef struct pthread_barrierattr_t pthread_barrierattr_t;
typedef struct pthread_cond_t pthread_cond_t;
typedef struct pthread_condattr_t pthread_condattr_t;
typedef struct pthread_key_t pthread_key_t;
typedef struct pthread_mutex_t pthread_mutex_t;
typedef struct pthread_mutexattr_t pthread_mutexattr_t;
typedef struct pthread_once_t pthread_once_t;
typedef struct pthread_rwlock_t pthread_rwlock_t;
typedef struct pthread_rwlockattr_t pthread_rwlockattr_t;
typedef struct pthread_spinlock_t {
    unsigned char lock[sizeof(void*)];
} pthread_spinlock_t;
typedef struct pthread_t {
    size_t id;
} pthread_t;


#endif /* __PHOENIX_SYS_TYPES_H */
