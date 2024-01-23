/* Copyright (c) 2019-2024 Jeremy Davis (jeremydavis519@gmail.com)
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

/* This file contains the C standard library's time-related functions and types. Since everything
 * here is standard, see http://www.cplusplus.com/reference/ctime/ for docs. */

#ifndef __PHOENIX_TIME_H
#define __PHOENIX_TIME_H

#include <locale.h>
#include <stddef.h>
#include <sys/types.h>

#define CLOCKS_PER_SEC 1000000

#define CLOCK_MONOTONIC             0
#define CLOCK_PROCESS_CPUTIME_ID    1
#define CLOCK_REALTIME              2
#define CLOCK_THREAD_CPUTIME_ID     3

#define TIMER_ABSTIME               1

#ifdef __cplusplus
extern "C" {
#endif

#if !defined(__cplusplus) && __STDC_VERSION__ >= 199901L
#define _PHOENIX_restrict restrict
#else
#define _PHOENIX_restrict
#endif /* __cplusplus and __STDC_VERSION__ */

struct sigevent;

struct tm {
    int tm_sec;
    int tm_min;
    int tm_hour;
    int tm_mday;
    int tm_mon;
    int tm_year;
    int tm_wday;
    int tm_yday;
    int tm_isdst;
};

struct timespec {
    time_t tv_sec;
    long   tv_nsec;
};

struct itimerspec {
    struct timespec it_interval;
    struct timespec it_value;
};

int getdate_err;

/* Measurement */
clock_t     clock(void);
time_t      time(time_t* time);
int         clock_getcpuclockid(pid_t pid, clockid_t* clock_id);
int         clock_getres(clockid_t clock_id, struct timespec* res);
int         clock_gettime(clockid_t clock_id, struct timespec* time);
int         clock_settime(clockid_t clock_id, struct timespec* time);
int         timer_create(clockid_t clock_id, struct sigevent* _PHOENIX_restrict event, timer_t* _PHOENIX_restrict timer_id);
int         timer_delete(timer_t timer_id);
int         timer_getoverrun(timer_t timer_id);
int         timer_gettime(timer_t timer_id, struct itimerspec* value);
int         timer_settime(
                timer_t                                    timer_id,
                int                                        flags,
                const struct itimerspec* _PHOENIX_restrict value,
                struct itimerspec* _PHOENIX_restrict       ovalue
            );

/* Time manipulation */
double      difftime(time_t end, time_t start);
time_t      mktime(struct tm* time);
struct tm*  getdate(const char* string);

/* Sleeping */
int         nanosleep(const struct timespec* until, struct timespec* remaining_time);
int         clock_nanosleep(clockid_t clock_id, int flags, const struct timespec* until, struct timespec* remaining_time);

/* Conversion */
char*       asctime(const struct tm* time);
char*       asctime_r(const struct tm* _PHOENIX_restrict time, char* _PHOENIX_restrict buf);
char*       ctime(const time_t* time);
char*       ctime_r(const time_t* time, char* buf);
struct tm*  gmtime(const time_t* time);
struct tm*  gmtime_r(const time_t* _PHOENIX_restrict time, struct tm* _PHOENIX_restrict result);
struct tm*  localtime(const time_t* time);
struct tm*  localtime_r(const time_t* _PHOENIX_restrict time, struct tm* _PHOENIX_restrict result);
size_t      strftime(char* _PHOENIX_restrict s, size_t maxsize, const char* _PHOENIX_restrict format, const struct tm* _PHOENIX_restrict time);
size_t      strftime_l(
                char* _PHOENIX_restrict            s,
                size_t                             maxsize,
                const char* _PHOENIX_restrict      format,
                const struct tm* _PHOENIX_restrict time,
                locale_t                           locale
            );
char*       strptime(const char* _PHOENIX_restrict buf, const char* _PHOENIX_restrict format, struct tm* _PHOENIX_restrict time);

/* Time zones */
extern int      daylight;
extern long     timezone;
extern char*    tzname[2];
void        tzset(void);

#ifdef __cplusplus
}
#endif

#endif /* __PHOENIX_TIME_H */
