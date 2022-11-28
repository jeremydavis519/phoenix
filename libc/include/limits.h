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

/* Macros and constants representing various POSIX-defined limits.
   https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/limits.h.html */

#ifndef __PHOENIX_LIMITS_H
#define __PHOENIX_LIMITS_H

#include <stdint.h>
#include <stdio.h>

/* Runtime invariant values (possibly indeterminate) */
/* Undefined means indeterminate */
/* TODO: #define AIO_LISTIO_MAX                 {>= _POSIX_AIO_LISTIO_MAX} */
/* TODO: #define AIO_MAX                        {>= _POSIX_AIO_MAX} */
/* TODO: #define AIO_PRIO_DELTA_MAX             {>= 0} */
/* TODO: #define ARG_MAX                        {>= _POSIX_ARG_MAX} */
/* TODO: #define ATEXIT_MAX                     {>= _POSIX_ATEXIT} */
/* TODO: #define CHILD_MAX                      {>= _POSIX_CHILD_MAX} */
/* TODO: #define DELAYTIMER_MAX                 {>= _POSIX_DELAYTIMER_MAX} */
/* TODO: #define HOST_NAME_MAX                  {>= _POSIX_HOST_NAME_MAX} */
/* TODO: #define IOV_MAX                        {>= _POSIX_IOV_MAX} */
/* TODO: #define LOGIN_NAME_MAX                 {>= _POSIX_LOGIN_NAME_MAX} */
/* TODO: #define MQ_OPEN_MAX                    {>= _POSIX_MQ_OPEN_MAX} */
/* TODO: #define MQ_PRIO_MAX                    {>= _POSIX_MQ_PRIO_MAX} */
/* TODO: #define OPEN_MAX                       {>= _POSIX_OPEN_MAX} */
/* #define PAGESIZE                             indeterminate */
#ifdef PAGESIZE
#define PAGE_SIZE                               PAGESIZE
#endif
/* TODO: #define PTHREAD_DESTRUCTOR_ITERATIONS  {>= _POSIX_THREAD_DESTRUCTOR_ITERATIONS} */
/* TODO: #define PTHREAD_KEYS_MAX               {>= _POSIX_THREAD_KEYS_MAX} */
/* TODO: #define PTHREAD_STACK_MIN              {>= 0} */
/* TODO: #define PTHREAD_THREADS_MAX            {>= _POSIX_THREADS_MAX} */
/* TODO: #define RTSIG_MAX                      {>= _POSIX_RTSIG_MAX} */
/* TODO: #define SEM_NSEMS_MAX                  {>= _POSIX_SEM_NSEMS_MAX} */
/* TODO: #define SEM_VALUE_MAX                  {>= _POSIX_SEM_VALUE_MAX} */
/* TODO: #define SIGQUEUE_MAX                   {>= _POSIX_SIGQUEUE_MAX} */
/* TODO: #define SS_REPL_MAX                    {>= _POSIX_SS_REPL_MAX} */
#define STREAM_MAX                              FOPEN_MAX
/* TODO: #define SYMLOOP_MAX                    {>= _POSIX_SYMLOOP_MAX} */
/* TODO: #define TIMER_MAX                      {>= _POSIX_TIMER_MAX} */
/* TODO: #define TRACE_EVENT_NAME_MAX           {>= _POSIX_TRACE_EVENT_NAME_MAX} */
/* TODO: #define TRACE_NAME_MAX                 {>= _POSIX_TRACE_NAME_MAX} */
/* TODO: #define TRACE_SYS_MAX                  {>= _POSIX_TRACE_SYS_MAX} */
/* TODO: #define TRACE_USER_EVENT_MAX           {>= _POSIX_TRACE_USER_EVENT_MAX} */
/* TODO: #define TTY_NAME_MAX                   {>= _POSIX_TTY_NAME_MAX} */
/* TODO: #define TZNAME_MAX                     {>= _POSIX_TZNAME_MAX} */

/* Pathname variable values (possibly indeterminate) */
/* Undefined means indeterminate */
/* TODO: #define FILESIZEBITS                   {>= 32} */
/* TODO: #define LINK_MAX                       {>= _POSIX_LINK_MAX} */
/* TODO: #define MAX_CANON                      {>= _POSIX_MAX_CANON} */
/* TODO: #define MAX_INPUT                      {>= _POSIX_MAX_INPUT} */
/* TODO: #define NAME_MAX                       {>= _POSIX_NAME_MAX and >= _XOPEN_NAME_MAX} */
/* TODO: #define PATH_MAX                       {>= _POSIX_PATH_MAX and >= _XOPEN_PATH_MAX} */
/* TODO: #define PIPE_BUF                       {>= _POSIX_PIPE_BUF} */
/* TODO: #define POSIX_ALLOC_SIZE_MIN           {no minimum acceptable value} */
/* TODO: #define POSIX_REC_INCR_XFER_SIZE       {no minimum acceptable value} */
/* TODO: #define POSIX_REC_MAX_XFER_SIZE        {no minimum acceptable value} */
/* TODO: #define POSIX_REC_MIN_XFER_SIZE        {no minimum acceptable value} */
/* TODO: #define POSIX_REC_XFER_ALIGN           {no minimum acceptable value} */
/* TODO: #define SYMLINK_MAX                    {>= _POSIX_SYMLINK_MAX} */

/* Runtime increasable values */
/* TODO: #define BC_BASE_MAX                    {>= _POSIX2_BC_BASE_MAX} */
/* TODO: #define BC_DIM_MAX                     {>= _POSIX2_BC_DIM_MAX} */
/* TODO: #define BC_SCALE_MAX                   {>= _POSIX2_BC_SCALE_MAX} */
/* TODO: #define BC_STRING_MAX                  {>= _POSIX2_BC_STRING_MAX} */
/* TODO: #define CHARCLASS_NAME_MAX             {>= _POSIX2_CHARCLASS_NAME_MAX} */
/* TODO: #define COLL_WEIGHTS_MAX               {>= _POSIX2_COLL_WEIGHTS_MAX} */
/* TODO: #define EXPR_NEST_MAX                  {>= _POSIX2_EXPR_NEST_MAX} */
/* TODO: #define LINE_MAX                       {>= _POSIX2_LINE_MAX} */
/* TODO: #define NGROUPS_MAX                    {>= _POSIX2_NGROUPS_MAX} */
/* TODO: #define RE_DUP_MAX                     {>= _POSIX2_RE_DUP_MAX} */

/* Maximum values for any POSIX implementation */
#define _POSIX_CLOCKRES_MIN                     20000000

/* Minimum values for any POSIX implementation */
#define _POSIX_AIO_LISTIO_MAX                   2
#define _POSIX_AIO_MAX                          1
#define _POSIX_ARG_MAX                          4096
#define _POSIX_CHILD_MAX                        25
#define _POSIX_DELAYTIMER_MAX                   32
#define _POSIX_HOST_NAME_MAX                    255
#define _POSIX_LINK_MAX                         8
#define _POSIX_LOGIN_NAME_MAX                   9
#define _POSIX_MAX_CANON                        255
#define _POSIX_MAX_INPUT                        255
#define _POSIX_MQ_OPEN_MAX                      8
#define _POSIX_MQ_PRIO_MAX                      32
#define _POSIX_NAME_MAX                         14
#define _POSIX_NGROUPS_MAX                      8
#define _POSIX_OPEN_MAX                         20
#define _POSIX_PATH_MAX                         256
#define _POSIX_PIPE_BUF                         512
#define _POSIX_RE_DUP_MAX                       255
#define _POSIX_RTSIG_MAX                        8
#define _POSIX_SEM_NSEMS_MAX                    256
#define _POSIX_SEM_VALUE_MAX                    32767
#define _POSIX_SIGQUEUE_MAX                     32
#define _POSIX_SSIZE_MAX                        32767
#define _POSIX_SS_REPL_MAX                      4
#define _POSIX_STREAM_MAX                       8
#define _POSIX_SYMLINK_MAX                      255
#define _POSIX_SYMLOOP_MAX                      8
#define _POSIX_THREAD_DESTRUCTOR_ITERATIONS     4
#define _POSIX_THREAD_KEYS_MAX                  128
#define _POSIX_THREAD_THREADS_MAX               64
#define _POSIX_TIMER_MAX                        32
#define _POSIX_TRACE_EVENT_NAME_MAX             30
#define _POSIX_TRACE_NAME_MAX                   8
#define _POSIX_TRACE_SYS_MAX                    8
#define _POSIX_TRACE_USER_EVENT_MAX             32
#define _POSIX_TTY_NAME_MAX                     9
#define _POSIX_TZNAME_MAX                       6
#define _POSIX2_BC_BASE_MAX                     99
#define _POSIX2_BC_DIM_MAX                      2048
#define _POSIX2_BC_SCALE_MAX                    99
#define _POSIX2_BC_STRING_MAX                   1000
#define _POSIX2_CHARCLASS_NAME_MAX              14
#define _POSIX2_COLL_WEIGHTS_MAX                2
#define _POSIX2_EXPR_NEST_MAX                   32
#define _POSIX2_LINE_MAX                        2048
#define _POSIX2_RE_DUP_MAX                      255
#define _XOPEN_IOV_MAX                          16
#define _XOPEN_NAME_MAX                         255
#define _XOPEN_PATH_MAX                         1024

/* Numerical limits (some are defined in stdint.h) */
#ifdef __GNUC__

#define CHAR_BIT                                __CHAR_BIT__
#define SCHAR_MAX                               __SCHAR_MAX__
#define SCHAR_MIN                               (-SCHAR_MAX - 1)
#define UCHAR_MAX                               ((1 << CHAR_BIT) - 1)
#ifdef __CHAR_UNSIGNED__
#define CHAR_MAX                                UCHAR_MAX
#define CHAR_MIN                                0
#else
#define CHAR_MAX                                SCHAR_MAX
#define CHAR_MIN                                SCHAR_MIN
#endif

#define SHRT_MAX                                __SHRT_MAX__
#define SHRT_MIN                                (-SHRT_MAX - 1)
#define USHRT_MAX                               ((1 << __SHRT_WIDTH__) - 1)

#define WORD_BIT                                __INT_WIDTH__
#define INT_MAX                                 __INT_MAX__
#define INT_MIN                                 (-INT_MAX - 1)
#define UINT_MAX                                ((1 << WORD_BIT) - 1)

#define LONG_BIT                                __LONG_WIDTH__
#define LONG_MAX                                __LONG_MAX__
#define LONG_MIN                                (-LONG_MAX - 1)
#define ULONG_MAX                               ((1 << LONG_BIT) - 1)

#define LLONG_MAX                               __LONG_LONG_MAX__
#define LLONG_MIN                               (-LLONG_MAX - 1)
#define ULLONG_MAX                              ((1 << __LONG_LONG_WIDTH__) - 1)

#else

#error "limits.h currently requires GCC."

#endif /* __GNUC__ */

#define SSIZE_MAX                               SIZE_MAX

#define MB_LEN_MAX                              4 /* Enough for UTF-8 */

/* Other invariant values */
/* TODO: #define NL_ARGMAX                      {>= 9} */
/* TODO: #define NL_LANGMAX                     {>= 14} */
/* TODO: #define NL_MSGMAX                      {>= 32767} */
/* TODO: #define NL_SETMAX                      {>= 255} */
/* TODO: #define NL_TEXTMAX                     {>= _POSIX2_LINE_MAX} */
/* TODO: #define NZERO                          {>= 20} */

#endif /* __PHOENIX_LIMITS_H */
