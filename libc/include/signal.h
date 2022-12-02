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

/* Definitions related to sending and receiving POSIX signals
   https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/signal.h.html */

#ifndef __PHOENIX_SIGNAL_H
#define __PHOENIX_SIGNAL_H

#include <time.h>
#include <sys/types.h>

/* TODO
#define SIG_DFL     ???
#define SIG_ERR     ???
#define SIG_HOLD    ???
#define SIG_IGN     ??? */

/* Possible values of `sigevent::sigev_notify` */
#define SIGEV_NONE      0
#define SIGEV_SIGNAL    1
#define SIGEV_THREAD    2

/* Range of signal values reserved for application use */
/* TODO
#define SIGRTMIN        ???
#define SIGRTMAX        ??? */

/* Standard signals */
/* 0 is reserved for the null signal. */
#define SIGABRT          1
#define SIGALRM          2
#define SIGBUS           3
#define SIGCHLD          4
#define SIGCONT          5
#define SIGFPE           6
#define SIGHUP           7
#define SIGILL           8
#define SIGINT           9
#define SIGKILL         10
#define SIGPIPE         11
#define SIGQUIT         12
#define SIGSEGV         13
#define SIGSTOP         14
#define SIGTERM         15
#define SIGTSTP         16
#define SIGTTIN         17
#define SIGTTOU         18
#define SIGUSR1         19
#define SIGUSR2         20
#define SIGPOLL         21
#define SIGPROF         22
#define SIGSYS          23
#define SIGTRAP         24
#define SIGURG          25
#define SIGVTALRM       26
#define SIGXCPU         27
#define SIGXFSZ         28

#define SIG_BLOCK       0
#define SIG_UNBLOCK     1
#define SIG_SETMASK     2

#define SA_NOCLDSTOP    0
#define SA_ONSTACK      1
#define SA_RESETHAND    2
#define SA_RESTART      3
#define SA_SIGINFO      4
#define SA_NOCLDWAIT    5
#define SA_NODEFER      6

#define SS_ONSTACK      0
#define SS_DISABLE      1

/* TODO
#define MINSIGSTKSZ     ???
#define SIGSTKSZ        ??? */

/* Reasons why a signal was generated */
#define ILL_ILLOPC      0x01
#define ILL_ILLOPN      0x02
#define ILL_ILLADR      0x03
#define ILL_ILLTRP      0x04
#define ILL_PRVOPC      0x05
#define ILL_PRVREG      0x06
#define ILL_COPROC      0x07
#define ILL_BADSTK      0x08
#define FPE_INTDIV      0x11
#define FPE_INTOVF      0x12
#define FPE_FLTDIV      0x13
#define FPE_FLTOVF      0x14
#define FPE_FLTUND      0x15
#define FPE_FLTRES      0x16
#define FPE_FLTINV      0x17
#define FPE_FLTSUB      0x18
#define SEGV_MAPERR     0x21
#define SEGV_ACCERR     0x22
#define BUS_ADRALN      0x31
#define BUS_ADRERR      0x32
#define BUS_OBJERR      0x33
#define TRAP_BRKPT      0x41
#define TRAP_TRACE      0x42
#define CLD_EXITED      0x51
#define CLD_KILLED      0x52
#define CLD_DUMPED      0x53
#define CLD_TRAPPED     0x54
#define CLD_STOPPED     0x55
#define CLD_CONTINUED   0x56
#define POLL_IN         0x61
#define POLL_OUT        0x62
#define POLL_MSG        0x63
#define POLL_ERR        0x64
#define POLL_PRI        0x65
#define POLL_HUP        0x66
#define SI_USER         -0x01
#define SI_QUEUE        -0x02
#define SI_TIMER        -0x03
#define SI_ASYNCIO      -0x04
#define SI_MESGQ        -0x05

#ifdef __cplusplus
extern "C" {
#endif /* __cplusplus */

#if defined(__cplusplus) || !defined(__STDC_VERSION__) || __STDC_VERSION__ < 199901L
#define restrict
#endif /* __cplusplus or __STDC_VERSION__ */

typedef __SIG_ATOMIC_TYPE__ sig_atomic_t;
typedef uint64_t            sigset_t;

union sigval {
    int     sival_int;
    void*   sival_ptr;
};

struct sigevent {
    int             sigev_notify;
    int             sigev_signo;
    union sigval    sigev_value;
    void          (*sigev_notify_function)(union sigval);
    pthread_attr_t* sigev_notify_attributes;
};

typedef struct siginfo_t {
    int             si_signo;
    int             si_code;
    int             si_errno;
    pid_t           si_pid;
    uid_t           si_uid;
    void*           si_addr;
    int             si_status;
    long            si_band;
    union sigval    si_value;
} siginfo_t;

struct sigaction {
    sigset_t    sa_mask;
    int         sa_flags;
#if __STDC_VERSION__ >= 199901L
    union {
#endif
        void  (*sa_handler)(int);
        void  (*sa_sigaction)(int, siginfo_t*, void*);
#if __STDC_VERSION__ >= 199901L
    };
#endif
};

/* TODO
typedef ??? mcontext_t;
typedef struct {
    ucontext_t* uc_link;
    sigset_t    uc_sigmask;
    stack_t     uc_stack;
    mcontext_t  uc_mcontext;
} ucontext_t; */

typedef struct {
    void*   ss_sp;
    size_t  ss_size;
    int     ss_flags;
} stack_t;

/* Sending signals */
int    kill(pid_t pid, int sig);
int    killpg(pid_t pgrp, int sig);
int    pthread_kill(pthread_t thread, int sig);
int    raise(int sig);
void (*signal(int sig, void (*func)(int)))(int);
int    sigqueue(pid_t pid, int sig, union sigval value);

/* Handling signals */
int    sigaction(int sig, const struct sigaction* restrict act, struct sigaction* restrict oact);
int    sigaltstack(const stack_t* restrict ss, stack_t* restrict oss);
int    siginterrupt(int sig, int flag);

/* Waiting for signals */
int    sigpending(sigset_t* set);
int    sigsuspend(const sigset_t* sigmask);
int    sigtimedwait(const sigset_t* restrict set, siginfo_t* restrict info, const struct timespec* restrict timeout);
int    sigwait(const sigset_t* restrict set, int* restrict sig);
int    sigwaitinfo(const sigset_t* restrict set, siginfo_t* restrict info);

/* Signal sets */
int    pthread_sigmask(int how, const sigset_t* restrict set, sigset_t* restrict oset);
int    sigprocmask(int how, const sigset_t* restrict set, sigset_t* restrict oset);
int    sigaddset(sigset_t* set, int sig);
int    sigdelset(sigset_t* set, int sig);
int    sigemptyset(sigset_t* set);
int    sigfillset(sigset_t* set);
int    sigismember(const sigset_t* set, int sig);

/* Signal management */
int    sighold(int sig);
int    sigignore(int sig);
int    sigpause(int sig);
int    sigrelse(int sig);
void (*sigset(int sig, void (*disp)(int)))(int);

/* Diagnostics */
void   psiginfo(const siginfo_t* pinfo, const char* message);
void   psignal(int sig, const char* message);

#if defined(__cplusplus) || !defined(__STDC_VERSION__) || __STDC_VERSION__ < 199901L
#undef restrict
#endif /* __cplusplus or __STDC_VERSION__ */

#ifdef __cplusplus
}
#endif /* __cplusplus */

#endif /* __PHOENIX_SIGNAL_H */
