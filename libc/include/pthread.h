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

#ifndef __PHOENIX_PTHREAD_H
#define __PHOENIX_PTHREAD_H

#include <sched.h>
#include <time.h>

#define PTHREAD_BARRIER_SERIAL_THREAD 1

#define PTHREAD_CANCEL_ENABLE         1
#define PTHREAD_CANCEL_DISABLE        2

#define PTHREAD_CANCEL_DEFERRED       1
#define PTHREAD_CANCEL_ASYNCHRONOUS   2

#define PTHREAD_CANCELED              ((void*)&_PHOENIX_unused)

#define PTHREAD_CREATE_DETACHED       1
#define PTHREAD_CREATE_JOINABLE       2

#define PTHREAD_EXPLICIT_SCHED        1
#define PTHREAD_INHERIT_SCHED         2

#define PTHREAD_MUTEX_DEFAULT         1
#define PTHREAD_MUTEX_ERRORCHECK      2
#define PTHREAD_MUTEX_NORMAL          3
#define PTHREAD_MUTEX_RECURSIVE       4
#define PTHREAD_MUTEX_ROBUST          5
#define PTHREAD_MUTEX_STALLED         6

#define PTHREAD_ONCE_INIT             1

#define PTHREAD_PRIO_NONE             1
#define PTHREAD_PRIO_INHERIT          2
#define PTHREAD_PRIO_PROTECT          3

#define PTHREAD_PROCESS_SHARED        1
#define PTHREAD_PROCESS_PRIVATE       2

#define PTHREAD_SCOPE_PROCESS         1
#define PTHREAD_SCOPE_SYSTEM          2

/* TODO: #define PTHREAD_COND_INITIALIZER {initializer for type pthread_cond_t} */
/* TODO: #define PTHREAD_MUTEX_INITIALIZER {initializer for type pthread_mutex_t} */
/* TODO: #define PTHREAD_RWLOCK_INITIALIZER {initializer for type pthread_rwlock_t} */

#ifdef __cplusplus
extern "C" {
#endif

#if !defined(__cplusplus) && __STDC_VERSION__ >= 199901L
#define _PHOENIX_restrict restrict
#else
#define _PHOENIX_restrict
#endif /* __cplusplus and __STDC_VERSION__ */

extern unsigned char _PHOENIX_unused; /* Any pointer to this byte is considered to be dangling. */

/* Threads */
pthread_t pthread_self(void);
int pthread_equal(pthread_t t1, pthread_t t2);
int pthread_create(pthread_t* _PHOENIX_restrict thread, const pthread_attr_t* _PHOENIX_restrict attr, void* (*start_routine)(void*), void* _PHOENIX_restrict arg);
void pthread_exit(void* result);
int pthread_join(pthread_t thread, void** result);
int pthread_detach(pthread_t thread);
int pthread_cancel(pthread_t thread);
void pthread_testcancel(void);
int pthread_atfork(void (*prepare)(void), void (*parent)(void), void (*child)(void));

struct _PHOENIX_pthread_cleanup_handler_node {
    void (*handler)(void*);
    void* arg;
    struct _PHOENIX_pthread_cleanup_handler_node* next;
};

extern __thread struct _PHOENIX_pthread_cleanup_handler_node* _PHOENIX_pthread_cleanup_handler_head;

/* void pthread_cleanup_push(void (*routine)(void*), void* arg); */
#define pthread_cleanup_push(routine, arg) do { \
    struct _PHOENIX_pthread_cleanup_handler_node __pthread_cleanup_handler_node; \
    __pthread_cleanup_handler_node.handler = routine; \
    __pthread_cleanup_handler_node.arg = arg; \
    __pthread_cleanup_handler_node.next = _PHOENIX_cleanup_handler_head; \
    do { _PHOENIX_pthread_cleanup_handler_head = &__pthread_cleanup_handler_node } while (0)

/* void pthread_cleanup_pop(int execute); */
#define pthread_cleanup_pop(execute) \
    *_PHOENIX_pthread_cleanup_handler_head = __pthread_cleanup_handler_node.next; \
    if (execute) (*__pthread_cleanup_handler_node.handler)(__pthread_cleanup_handler_node.arg); \
} while (0)

/* Thread attributes */
int pthread_setcancelstate(int state, int* oldstate);
int pthread_setcanceltype(int type, int* oldtype);
int pthread_setconcurrency(int new_level);
int pthread_getconcurrency();
int pthread_getcpuclockid(pthread_t thread, clockid_t* clock);
int pthread_setschedparam(pthread_t thread, int policy, const struct sched_param* schedparam);
int pthread_getschedparam(pthread_t thread, int* _PHOENIX_restrict policy, struct sched_param* _PHOENIX_restrict schedparam);
int pthread_setschedprio(pthread_t, int schedprio);
int pthread_key_create(pthread_key_t* key, void (*destructor)(void*));
int pthread_key_delete(pthread_key_t key);
int pthread_setspecific(pthread_key_t key, const void* value);
void* pthread_getspecific(pthread_key_t key);
int pthread_attr_init(pthread_attr_t* attr);
int pthread_attr_destroy(pthread_attr_t* attr);
int pthread_attr_setdetachstate(pthread_attr_t* attr, int detachstate);
int pthread_attr_getdetachstate(const pthread_attr_t* attr, int* detachstate);
int pthread_attr_setguardsize(pthread_attr_t* attr, size_t guardsize);
int pthread_attr_getguardsize(const pthread_attr_t* _PHOENIX_restrict attr, size_t* _PHOENIX_restrict guardsize);
int pthread_attr_setinheritsched(pthread_attr_t* attr, int inheritsched);
int pthread_attr_getinheritsched(const pthread_attr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict inheritsched);
int pthread_attr_setschedparam(pthread_attr_t* _PHOENIX_restrict attr, const struct sched_param* _PHOENIX_restrict schedparam);
int pthread_attr_getschedparam(const pthread_attr_t* _PHOENIX_restrict attr, struct sched_param* _PHOENIX_restrict schedparam);
int pthread_attr_setschedpolicy(pthread_attr_t* attr, int schedpolicy);
int pthread_attr_getschedpolicy(const pthread_attr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict schedpolicy);
int pthread_attr_setscope(pthread_attr_t* attr, int scope);
int pthread_attr_getscope(const pthread_attr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict scope);
int pthread_attr_setstack(pthread_attr_t* attr, void* stackptr, size_t stacksize);
int pthread_attr_getstack(const pthread_attr_t* _PHOENIX_restrict attr, void** _PHOENIX_restrict stackptr, size_t* _PHOENIX_restrict stacksize);
int pthread_attr_setstacksize(pthread_attr_t* attr, size_t stacksize);
int pthread_attr_getstacksize(const pthread_attr_t* _PHOENIX_restrict attr, size_t* _PHOENIX_restrict stacksize);

/* Spinlocks */
int pthread_spin_init(pthread_spinlock_t* lock, int pshared);
int pthread_spin_destroy(pthread_spinlock_t* lock);
int pthread_spin_lock(pthread_spinlock_t* lock);
int pthread_spin_trylock(pthread_spinlock_t* lock);
int pthread_spin_unlock(pthread_spinlock_t* lock);

/* Mutexes */
int pthread_mutex_init(pthread_mutex_t* _PHOENIX_restrict mutex, const pthread_mutexattr_t* _PHOENIX_restrict attr);
int pthread_mutex_destroy(pthread_mutex_t* mutex);
int pthread_mutex_lock(pthread_mutex_t* mutex);
int pthread_mutex_timedlock(pthread_mutex_t* _PHOENIX_restrict mutex, const struct timespec* _PHOENIX_restrict abstime);
int pthread_mutex_trylock(pthread_mutex_t* mutex);
int pthread_mutex_unlock(pthread_mutex_t* mutex);
int pthread_mutex_setprioceiling(pthread_mutex_t* _PHOENIX_restrict mutex, int prioceiling, int* _PHOENIX_restrict old_ceiling);
int pthread_mutex_getprioceiling(const pthread_mutex_t* _PHOENIX_restrict mutex, int* _PHOENIX_restrict prioceiling);
int pthread_mutex_consistent(pthread_mutex_t* mutex);
int pthread_mutexattr_init(pthread_mutexattr_t* attr);
int pthread_mutexattr_destroy(pthread_mutexattr_t* attr);
int pthread_mutexattr_setprioceiling(pthread_mutexattr_t* attr, int prioceiling);
int pthread_mutexattr_getprioceiling(const pthread_mutexattr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict prioceiling);
int pthread_mutex_attr_setprotocol(pthread_mutexattr_t* attr, int protocol);
int pthread_mutex_attr_getprotocol(const pthread_mutexattr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict protocol);
int pthread_mutexattr_setpshared(pthread_mutexattr_t* attr, int pshared);
int pthread_mutexattr_getpshared(const pthread_mutexattr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict pshared);
int pthread_mutexattr_setrobust(pthread_mutexattr_t* attr, int robust);
int pthread_mutexattr_getrobust(const pthread_mutexattr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict robust);
int pthread_mutexattr_settype(pthread_mutexattr_t* attr, int type);
int pthread_mutexattr_gettype(const pthread_mutexattr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict type);

/* RW Locks */
int pthread_rwlock_init(pthread_rwlock_t* _PHOENIX_restrict lock, const pthread_rwlockattr_t* _PHOENIX_restrict attr);
int pthread_rwlock_destroy(pthread_rwlock_t* lock);
int pthread_rwlock_rdlock(pthread_rwlock_t* lock);
int pthread_rwlock_timedrdlock(pthread_rwlock_t* _PHOENIX_restrict lock, const struct timespec* _PHOENIX_restrict abstime);
int pthread_rwlock_tryrdlock(pthread_rwlock_t* lock);
int pthread_rwlock_wrlock(pthread_rwlock_t* lock);
int pthread_rwlock_timedwrlock(pthread_rwlock_t* _PHOENIX_restrict lock, const struct timespec* _PHOENIX_restrict abstime);
int pthread_rwlock_trywrlock(pthread_rwlock_t* lock);
int pthread_rwlock_unlock(pthread_rwlock_t* lock);
int pthread_rwlockattr_init(pthread_rwlockattr_t* attr);
int pthread_rwlockattr_destroy(pthread_rwlockattr_t* attr);
int pthread_rwlockattr_setpshared(pthread_rwlockattr_t* attr, int pshared);
int pthread_rwlockattr_getpshared(const pthread_rwlockattr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict pshared);

/* Barriers */
int pthread_barrier_init(pthread_barrier_t* _PHOENIX_restrict barrier, const pthread_barrierattr_t* _PHOENIX_restrict attr, unsigned int count);
int pthread_barrier_destroy(pthread_barrier_t* barrier);
int pthread_barrier_wait(pthread_barrier_t* barrier);
int pthread_barrierattr_init(pthread_barrierattr_t* attr);
int pthread_barrierattr_destroy(pthread_barrierattr_t* attr);
int pthread_barrierattr_setpshared(pthread_barrierattr_t* attr, int pshared);
int pthread_barrierattr_getpshared(const pthread_barrierattr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict pshared);

/* Condition variables */
int pthread_cond_init(pthread_cond_t* _PHOENIX_restrict cond, const pthread_condattr_t* _PHOENIX_restrict attr);
int pthread_cond_destroy(pthread_cond_t* cond);
int pthread_cond_signal(pthread_cond_t* cond);
int pthread_cond_broadcast(pthread_cond_t* cond);
int pthread_cond_wait(pthread_cond_t* _PHOENIX_restrict cond, pthread_mutex_t* _PHOENIX_restrict mutex);
int pthread_timedwait(pthread_cond_t* _PHOENIX_restrict cond, pthread_mutex_t* _PHOENIX_restrict mutex, const struct timespec* _PHOENIX_restrict abstime);
int pthread_condattr_init(pthread_condattr_t* attr);
int pthread_condattr_destroy(pthread_condattr_t* attr);
int pthread_condattr_setclock(pthread_condattr_t* attr, clockid_t clock);
int pthread_condattr_getclock(const pthread_condattr_t* _PHOENIX_restrict attr, clockid_t* _PHOENIX_restrict clock);
int pthread_condattr_setpshared(pthread_condattr_t* attr, int pshared);
int pthread_condattr_getpshared(const pthread_condattr_t* _PHOENIX_restrict attr, int* _PHOENIX_restrict pshared);

/* Once */
int pthread_once(pthread_once_t* once_control, void (*init_routine)(void));

#ifdef __cplusplus
}
#endif

#endif /* __PHOENIX_PTHREAD_H */
