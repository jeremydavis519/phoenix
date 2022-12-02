/* Copyright (c) 2021-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

/* POSIX-conforming declarations for waiting
   https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/sys_wait.h.html */

#ifndef __PHOENIX_SYS_WAIT_H
#define __PHOENIX_SYS_WAIT_H

#include <signal.h>
#include <sys/types.h>

/* For use with `waitpid()` */
#define WCONTINUED      0x01 /* also valid for `waitid()` */
#define WNOHANG         0x02 /* also valid for `waitid()` */
#define WUNTRACED       0x04

/* For use with `waitid()` */
#define WEXITED         0x08
#define WNOWAIT         0x10
#define WSTOPPED        0x20

/* For analysis of process status values */
#define WEXITSTATUS(stat)   ((stat) & 0xff)
#define WIFCONTINUED(stat)  ((((unsigned int)(stat) >> 8) & 0x03) == 0)
#define WIFEXITED(stat)     ((((unsigned int)(stat) >> 8) & 0x03) == 1)
#define WIFSIGNALED(stat)   ((((unsigned int)(stat) >> 8) & 0x03) == 2)
#define WIFSTOPPED(stat)    ((((unsigned int)(stat) >> 8) & 0x03) == 3)
/* Weird typecasts to get around implementation-defined right shifts on signed numbers */
#define WSTOPSIG(stat)      ((int)((uintmax_t)(intmax_t)(stat) >> 10))
#define WTERMSIG(stat)      ((int)((uintmax_t)(intmax_t)(stat) >> 10))

#ifdef __cplusplus
extern "C" {
#endif /* __cplusplus */

typedef enum {
    P_ALL  = 0,
    P_PGID = 1,
    P_PID  = 2,
} idtype_t;

pid_t   wait(int* stat_loc);
int     waitid(idtype_t idtype, id_t id, siginfo_t* infop, int options);
pid_t   waitpid(pid_t pid, int* stat_loc, int options);

#ifdef __cplusplus
}
#endif /* __cplusplus */

#endif /* __PHOENIX_SYS_WAIT_H */
