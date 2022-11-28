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

/* TODO
typedef  blkcnt_t;
typedef  blksize_t;
typedef  fsblkcnt_t;
typedef  fsfilcnt_t;
typedef  ino_t;
typedef  key_t;
typedef  mode_t;
typedef  nlink_t; */

#ifdef INT64_MAX
typedef int64_t off_t;
#define OFF_MIN INT64_MIN
#define OFF_MAX INT64_MAX
#else
typedef int32_t off_t;
#define OFF_MIN INT32_MIN
#define OFF_MAX INT32_max
#endif /* defined(INT64_MAX) */

typedef size_t ssize_t;

/* TODO
typedef  clockid_t;
typedef  clock_t;
typedef  suseconds_t;
typedef  time_t;
typedef  timer_t; */

typedef uint64_t id_t;
typedef uint32_t gid_t;
typedef uint32_t uid_t;
typedef uint32_t pid_t;

/* TODO
typedef  pthread_attr_t;
typedef  pthread_barrier_t;
typedef  pthread_barrierattr_t;
typedef  pthread_cond_t;
typedef  pthread_condattr_t;
typedef  pthread_key_t;
typedef  pthread_mutex_t;
typedef  pthread_mutexattr_t;
typedef  pthread_once_t;
typedef  pthread_rwlock_t;
typedef  pthread_rwlockattr_t;
typedef  pthread_spinlock_t;
typedef  pthread_t; */

#endif /* __PHOENIX_SYS_TYPES_H */
