/* Copyright (c) 2021 Jeremy Davis (jeremydavis519@gmail.com)
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

#ifndef _STDLIB_H
#define _STDLIB_H

#include <stddef.h>
#include <wchar.h>

#define EXIT_FAILURE -1
#define EXIT_SUCCESS 0
/* TODO: #define MB_CUR_MAX ... (should be the maximum size of a multibyte character in the current locale) */
#define RAND_MAX 37767

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    int quot;
    int rem;
} div_t;

typedef struct {
    long int quot;
    long int rem;
} ldiv_t;

/* String conversion */
double atof(const char* str);
int atoi(const char* str);
long int atol(const char* str);
double strtod(const char* str, char** endptr);
long int strtol(const char* str, char** endptr, int base);
unsigned long int strtoul(const char* str, char** endptr, int base);

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
int mbtowc(wchar_t* wc, const char* mbc, size_t max_bytes);
int wctomb(char* mbc, wchar_t wc);

/* Multibyte strings */
size_t mbstowcs(wchar_t* dest, const char* src, size_t max_chars);
size_t wcstombs(char* dest, const wchar_t *src, size_t max_bytes);

#ifdef __cplusplus
}
#endif

#endif /* _STDLIB_H */
