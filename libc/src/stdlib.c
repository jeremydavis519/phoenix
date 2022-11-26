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

#include <stdlib.h>

/* String conversion */
/* TODO
double atof(const char* str);
int atoi(const char* str);
long int atol(const char* str);
double strtod(const char* str, char** endptr);
long int strtol(const char* str, char** endptr, int base);
unsigned long int strtoul(const char* str, char** endptr, int base); */


/* Pseudorandom number generation */
static unsigned int _rand_seed = 0;

int rand(void) {
    _rand_seed = 65854829 * _rand_seed + 1;
    return _rand_seed / 65536 % (RAND_MAX + 1);
}

void srand(unsigned int seed) {
    _rand_seed = seed;
}


/* Dynamic memory management */
/* TODO
void* calloc(size_t num, size_t size);
void free(void* ptr);
void* malloc(size_t size);
void* realloc(void* ptr, size_t size); */


/* Environment */
/* TODO
void abort(void);
int atexit(void (*func)(void)); */

void exit(int status) {
    /* FIXME: All this has to be done before exiting the program:
    - Functions registered with atexit are called.
    - All C streams (open with functions in <cstdio>) are closed (and flushed, if buffered), and all files
      created with tmpfile are removed.
    - Control is returned to the host environment. */
    register int st asm ("x2") = status;
    asm volatile("svc 0x0100" :: "r"(st));
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
int mbtowc(wchar_t* wc, const char* mbc, size_t max_bytes);
int wctomb(char* mbc, wchar_t wc); */


/* Multibyte strings */
/* TODO
size_t mbstowcs(wchar_t* dest, const char* src, size_t max_chars);
size_t wcstombs(char* dest, const wchar_t *src, size_t max_bytes); */
