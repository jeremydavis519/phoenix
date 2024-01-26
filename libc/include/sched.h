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

#ifndef __PHOENIX_SCHED_H
#define __PHOENIX_SCHED_H

#include <sys/types.h>
#include <time.h>

#define SCHED_FIFO      1
#define SCHED_RR        2
#define SCHED_SPORADIC  3
#define SCHED_OTHER     4

#ifdef __cplusplus
extern "C" {
#endif

struct sched_param {
    int             sched_priority;
    int             sched_ss_low_priority;
    struct timespec sched_ss_repl_period;
    struct timespec sched_ss_init_budget;
    int             sched_ss_max_repl;
};

int sched_get_priority_max(int policy);
int sched_get_priority_min(int policy);
int sched_setparam(pid_t pid, const struct sched_param* param);
int sched_getparam(pid_t pid, struct sched_param* param);
int sched_setscheduler(pid_t pid, int policy, const struct sched_param* param);
int sched_getscheduler(pid_t pid);
int sched_rr_get_interval(pid_t pid, struct timespec* interval);
int sched_yield(void);

#ifdef __cplusplus
}
#endif

#endif /* __PHOENIX_SCHED_H */
