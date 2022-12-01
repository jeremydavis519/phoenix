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

#ifndef __PHOENIX_STDLIB_H
#define __PHOENIX_STDLIB_H

#include <stddef.h>
#include <wchar.h>

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

/* String conversion */
double atof(const char* str);
int atoi(const char* str);
long atol(const char* str);
float strtof(const char* restrict str, char** restrict endptr);
double strtod(const char* restrict str, char** restrict endptr);
long strtol(const char* restrict str, char** restrict endptr, int base);
unsigned long strtoul(const char* restrict str, char** restrict endptr, int base);
unsigned long long strtoull(const char* restrict str, char** restrict endptr, int base);

/* Pseudorandom number generation */
int rand(void);
void srand(unsigned int seed);

/* Dynamic memory management */
void* calloc(size_t num, size_t size);
void free(void* ptr);
void* malloc(size_t size);
void* realloc(void* ptr, size_t size);

/* Environment */
void abort(void);
int atexit(void (*func)(void));
void exit(int status);
char* getenv(const char* name);
int system(const char* command);

/* Searching and sorting */
void* bsearch(const void* key, const void* base, size_t num, size_t size, int (*compar)(const void*, const void*));
void qsort(void* base, size_t num, size_t size, int (*compar)(const void*, const void*));

/* Integer arithmetics */
int abs(int n);
div_t div(int numer, int denom);
long int labs(long int n);
ldiv_t ldiv(long int numer, long int denom);

/* Multibyte characters */
int mblen(const char* mbc, size_t max_bytes);
int mbtowc(wchar_t* restrict wc, const char* restrict mbc, size_t max_bytes);
int wctomb(char* mbc, wchar_t wc);

/* Multibyte strings */
size_t mbstowcs(wchar_t* restrict dest, const char* restrict src, size_t max_chars);
size_t wcstombs(char* restrict dest, const wchar_t* restrict src, size_t max_bytes);

#ifdef __cplusplus
#undef restrict
}
#endif

#endif /* __PHOENIX_STDLIB_H */
