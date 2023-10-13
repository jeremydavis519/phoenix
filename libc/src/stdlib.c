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

#include <stdlib.h>
#include <stdnoreturn.h>
#include <phoenix.h>

/* String conversion */
/* TODO
double atof(const char* str);
int atoi(const char* str);
long int atol(const char* str);
float strtof(const char* restrict str, char** restrict endptr);
double strtod(const char* restrict str, char** restrict endptr);
long strtol(const char* restrict str, char** restrict endptr, int base);
unsigned long strtoul(const char* restrict str, char** restrict endptr, int base);
unsigned long long strtoull(const char* restrict str, char** restrict endptr, int base); */


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


/* Dynamic memory management */
/* TODO
void* calloc(size_t num, size_t size); */

/* void free(void* ptr); (Defined in libphoenix) */
/* void* malloc(size_t size); (Defined in libphoenix) */

/* TODO
void* realloc(void* ptr, size_t size); */


/* Environment */
/* TODO
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
ldiv_t ldiv(long int numer, long int denom); */


/* Multibyte characters */
/* TODO
int mblen(const char* mbc, size_t max_bytes);
int mbtowc(wchar_t* restrict wc, const char* restrict mbc, size_t max_bytes);
int wctomb(char* mbc, wchar_t wc); */


/* Multibyte strings */
/* TODO
size_t mbstowcs(wchar_t* restrict dest, const char* restrict src, size_t max_chars);
size_t wcstombs(char* dest, const wchar_t* src, size_t max_bytes); */
