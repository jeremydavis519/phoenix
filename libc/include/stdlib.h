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

/* Standard library definitions
   https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/stdlib.h.html */

#ifndef __PHOENIX_STDLIB_H
#define __PHOENIX_STDLIB_H

#include <stddef.h>
#include <wchar.h>
#include <sys/wait.h>

#define EXIT_FAILURE -1
#define EXIT_SUCCESS 0
/* TODO: #define MB_CUR_MAX ... (should be the maximum size of a multibyte character in the current locale) */
#define RAND_MAX 32767

#ifdef __cplusplus
extern "C" {
#define restrict
#endif

typedef struct {
    int quot;
    int rem;
} div_t;

typedef struct {
    long quot;
    long rem;
} ldiv_t;

typedef struct {
    long long quot;
    long long rem;
} lldiv_t;

/* String conversion */
long                a64l(const char* s);
double              atof(const char* str);
int                 atoi(const char* str);
long                atol(const char* str);
long long           atoll(const char* str);
char*               l64a(long value);
float               strtof(const char* restrict str, char** restrict endptr);
double              strtod(const char* restrict str, char** restrict endptr);
long double         strtold(const char* restrict str, char** restrict endptr);
long                strtol(const char* restrict str, char** restrict endptr, int base);
long long           strtoll(const char* restrict str, char** restrict endptr, int base);
unsigned long       strtoul(const char* restrict str, char** restrict endptr, int base);
unsigned long long  strtoull(const char* restrict str, char** restrict endptr, int base);

/* Pseudorandom number generation */
int                 rand(void);
int                 rand_r(unsigned int* seed);
void                srand(unsigned int seed);
double              drand48(void);
double              erand48(unsigned short x[3]);
long                jrand48(unsigned short x[3]);
void                lcong48(unsigned short param[7]);
long                lrand48(void);
long                mrand48(void);
long                nrand48(unsigned short x[3]);
unsigned short*     seed48(unsigned short seed16v[3]);
void                srand48(long seed);
char*               initstate(unsigned int seed, char* state, size_t size);
long                random(void);
char*               setstate(char* state);
void                srandom(unsigned int seed);

/* Dynamic memory management */
void*               calloc(size_t num, size_t size);
void                free(void* ptr);
void*               malloc(size_t size);
void*               realloc(void* ptr, size_t size);
int                 posix_memalign(void** memptr, size_t alignment, size_t size);

/* Environment */
void                abort(void);
int                 atexit(void (*func)(void));
void                exit(int status);
void                _Exit(int status);
char*               getenv(const char* name);
int                 getsubopt(char** option, char* const* keylist, char** value);
int                 putenv(char* string);
int                 setenv(const char* name, const char* value, int overwrite);
int                 unsetenv(const char* name);
int                 system(const char* command);

/* Searching and sorting */
void*               bsearch(const void* key, const void* base, size_t num, size_t size, int (*compar)(const void*, const void*));
void                qsort(void* base, size_t num, size_t size, int (*compar)(const void*, const void*));

/* Integer arithmetics */
int                 abs(int n);
div_t               div(int numer, int denom);
long                labs(long n);
ldiv_t              ldiv(long numer, long denom);
long long           llabs(long long n);
lldiv_t             lldiv(long long numer, long long denom);

/* Multibyte characters */
int                 mblen(const char* mbc, size_t max_bytes);
int                 mbtowc(wchar_t* restrict wc, const char* restrict mbc, size_t max_bytes);
int                 wctomb(char* mbc, wchar_t wc);

/* Multibyte strings */
size_t              mbstowcs(wchar_t* restrict dest, const char* restrict src, size_t max_chars);
size_t              wcstombs(char* restrict dest, const wchar_t* restrict src, size_t max_bytes);

/* File system */
char*               mkdtemp(char* template);
int                 mkstemp(char* template);
char*               realpath(const char* restrict filename, char* restrict resolved_name);

/* Pseudo-terminals */
int                 grantpt(int fildes);
int                 posix_openpt(int oflags);
char*               ptsname(int fildes);
int                 unlockpt(int fildes);

/* Cryptography */
void                setkey(const char* key);

#ifdef __cplusplus
#undef restrict
}
#endif

#endif /* __PHOENIX_STDLIB_H */
