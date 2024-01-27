/* Copyright (c) 2021-2024 Jeremy Davis (jeremydavis519@gmail.com)
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

#include <ctype.h>
#include <errno.h>
#include <limits.h>
#include <stdlib.h>
#include <stdnoreturn.h>
#include <phoenix.h>

/* String conversion */
static unsigned long long parse_ull(const char* restrict str, size_t i, char** restrict endptr, int base);

/* TODO
long a64l(const char* s);
char* l64a(long value);
double atof(const char* str);
float strtof(const char* restrict str, char** restrict endptr);
double strtod(const char* restrict str, char** restrict endptr);
long double strtold(const char* restrict str, char** restrict endptr); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/atoi.html */
int atoi(const char* str) {
    return (int)strtol(str, NULL, 10);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/atol.html */
long int atol(const char* str) {
    return strtol(str, NULL, 10);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/atol.html */
long long atoll(const char* str) {
    return strtoll(str, NULL, 10);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/strtol.html */
long strtol(const char* restrict str, char** restrict endptr, int base) {
    long long result = strtoll(str, endptr, base);
    if (result > (long long)LONG_MAX) {
        errno = ERANGE;
        return LONG_MAX;
    }
    if (result < (long long)LONG_MIN) {
        errno = ERANGE;
        return LONG_MIN;
    }
    return (long)result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/strtoll.html */
long long strtoll(const char* restrict str, char** restrict endptr, int base) {
    size_t i = 0;
    while (isspace(str[i])) ++i;

    long long sign = 1;

    if (str[i] == '+') ++i;
    else if (str[i] == '-') {
        ++i;
        sign = -1;
    }

    unsigned long long magnitude = parse_ull(str, i, endptr, base);
    if (sign == 1 && magnitude > (unsigned long long)LLONG_MAX) {
        errno = ERANGE;
        return LLONG_MAX;
    }
    /* sign = -1 && magnitude > (unsigned long long)-LLONG_MIN, but guaranteed to work with any signed integer
     * representation as long as -LLONG_MAX is representable */
    if (sign == -1 && magnitude > (unsigned long long)LLONG_MAX + (unsigned long long)(-LLONG_MAX - LLONG_MIN)) {
        errno = ERANGE;
        return LLONG_MIN;
    }
    return sign * magnitude;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/strtoul.html */
unsigned long strtoul(const char* restrict str, char** restrict endptr, int base) {
    unsigned long long result = strtoull(str, endptr, base);
    if (result > (unsigned long long)ULONG_MAX) {
        errno = ERANGE;
        return ULONG_MAX;
    }
    return (unsigned long)result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/strtoull.html */
unsigned long long strtoull(const char* restrict str, char** restrict endptr, int base) {
    size_t i = 0;
    while (isspace(str[i])) ++i;

    if (str[i] == '+') ++i;
    else if (str[i] == '-') {
        ++i;
        errno = ERANGE; /* This will be overwritten with EINVAL if the parsing fails. */
        unsigned long long result = parse_ull(str, i, endptr, base);
        if (errno == ERANGE) return ULLONG_MAX;
        return result;
    }

    return parse_ull(str, i, endptr, base);
}

/* Implements strtoull, except that leading whitespace, '+', or '-' is an error. */
static unsigned long long parse_ull(const char* restrict str, size_t i, char** restrict endptr, int base) {
    if (base == 1 || base > 36) {
        errno = EINVAL;
        return 0;
    }

    if (base == 0) {
        if (str[i] == '0') {
            if (str[i + 1] == 'x' || str[i + 1] == 'X') {
                base = 16;
                i += 2;
            } else {
                base = 8;
            }
        } else {
            base = 10;
        }
    } else if (base == 16 && str[i] == '0' && (str[i + 1] == 'x' || str[i + 1] == 'X')) {
        i += 2;
    }

    unsigned long long value = ULLONG_MAX;
    if (str[i] >= '0' && str[i] <= '9') value = str[i] - '0';
    else if (str[i] >= 'A' && str[i] <= 'Z') value = str[i] - 'A' + 10;
    else if (str[i] >= 'a' && str[i] <= 'z') value = str[i] - 'a' + 10;
    ++i;

    if ((int)value >= base) {
        /* The first character in the number is invalid.
         * "If the subject sequence is empty or does not have the expected form, no conversion shall be performed;
         * The value of str shall be stored in the object pointed to by endptr, provided that endptr is not a null pointer." */
        if (endptr) *endptr = (char*)str;
        errno = EINVAL;
        return 0;
    }

    for (;; ++i) {
        int digit = INT_MAX;
        if (str[i] >= '0' && str[i] <= '9') digit = str[i] - '0';
        else if (str[i] >= 'A' && str[i] <= 'Z') value = str[i] - 'A' + 10;
        else if (str[i] >= 'a' && str[i] <= 'z') value = str[i] - 'a' + 10;

        if (digit >= base) break;

        unsigned long long new_value = value * base + digit;

        /* Saturate at ULLONG_MAX instead of wrapping. */
        if (new_value >= value) value = new_value;
        else value = ULLONG_MAX;
    }

    if (endptr) *endptr = (char*)str + i;
    return value;
}


/* Pseudorandom number generation */
static uint32_t rand_seed = 1;

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/rand.html */
int rand(void) {
    return rand_r(&rand_seed);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/rand_r.html */
int rand_r(unsigned int* seed) {
    *seed = 65854829 * *seed + 1;
    return *seed / (0xffffffff / (RAND_MAX + 1) + 1);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/srand.html */
void srand(unsigned int seed) {
    rand_seed = seed;
}

/* TODO
double drand48(void);
double erand48(unsigned short x[3]);
long jrand48(unsigned short x[3]);
void lcong48(unsigned short param[7]);
long lrand48(void);
long mrand48(void);
long nrand48(unsigned short x[3]);
unsigned short seed48(unsigned short seed16v[3]);
void srand48(long seed);
char* initstate(unsigned int seed, char* state, size_t size);
long random(void);
char* setstate(char* state);
void srandom(unsigned int seed); */


/* Dynamic memory management */
/* void* malloc(size_t size); (Defined in libphoenix) */
/* void* calloc(size_t num, size_t size); (Defined in libphoenix) */
/* void* realloc(void* ptr, size_t size); (Defined in libphoenix) */
/* void free(void* ptr); (Defined in libphoenix) */
/* TODO
int posix_memalign(void** memptr, size_t alignment, size_t size); */


/* Environment */
/* TODO
noreturn
void abort(void);
int atexit(void (*func)(void)); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/exit.html */
noreturn
void exit(int status) {
    /* FIXME: All this has to be done before exiting the program:
    - Functions registered with atexit are called.
    - All C streams (open with functions in <cstdio>) are closed (and flushed, if buffered), and all files
      created with tmpfile are removed. */
    _Exit(status);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/_Exit.html */
noreturn
void _Exit(int status) {
    /* FIXME: Implement all the "Consequences of Process Termination" at this link:
       https://pubs.opengroup.org/onlinepubs/9699919799/functions/_Exit.html#tag_16_01_03_01 */
    _PHOENIX_process_exit(status);
    while (1);
}

/* TODO
char* getenv(const char* name);
int getsubopt(char** option, char* const* keylist, char** value);
int putenv(char* string);
int setenv(const char* name, const char* value, int overwrite);
int unsetenv(const char* name);
int system(const char* command); */


/* Searching and sorting */
/* TODO
void* bsearch(const void* key, const void* base, size_t num, size_t size, int (*compar)(const void*, const void*));
void qsort(void* base, size_t num, size_t size, int (*compar)(const void*, const void*)); */


/* Integer arithmetics */
/* TODO
int abs(int n);
div_t div(int numer, int denom);
long int labs(long int n);
long long llabs(long long n);
ldiv_t ldiv(long int numer, long int denom);
lldiv_t lldiv(long long numer, long long denom); */


/* Multibyte characters */
/* TODO
int mblen(const char* mbc, size_t max_bytes);
int mbtowc(wchar_t* restrict wc, const char* restrict mbc, size_t max_bytes);
int wctomb(char* mbc, wchar_t wc); */


/* Multibyte strings */
/* TODO
size_t mbstowcs(wchar_t* restrict dest, const char* restrict src, size_t max_chars);
size_t wcstombs(char* dest, const wchar_t* src, size_t max_bytes); */


/* File system */
/* TODO
char* mkdtemp(char* template);
int mkstemp(char* template);
char* realpath(const char* restrict filename, char* restrict resolved_name); */


/* Pseudo-terminals */
/* TODO
int grantpt(int fildes);
int posix_openpt(int oflags);
char* ptsname(int fildes);
int unlockpt(int fildes); */


/* Cryptography */
/* TODO
void setkey(const char* key); */
