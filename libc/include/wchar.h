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

#ifndef _WCHAR_H
#define _WCHAR_H

#include <stddef.h>
#include <stdio.h>
#include <time.h>

#define WCHAR_MIN __WCHAR_MIN__
#define WCHAR_MAX __WCHAR_MAX__
#define WEOF -1

#ifdef __cplusplus
extern "C" {
#define restrict
#endif

typedef struct mbstate_t mbstate_t;
typedef wchar_t wint_t;

/* Input/output (mirroring stdio.h) */
wint_t fgetwc(FILE* stream);
wchar_t* fgetws(wchar_t* restrict ws, int max_chars, FILE* restrict stream);
wint_t fputwc(wchar_t wc, FILE* stream);
int fputws(const wchar_t* restrict ws, FILE* restrict stream);
int fwide(FILE* stream, int mode);
int fwprintf(FILE* restrict stream, const wchar_t* restrict format, ...);
int fwscanf(FILE* restrict stream, const wchar_t* restrict format, ...);
wint_t getwc(FILE* stream);
wint_t getwchar(void);
wint_t putwc(wchar_t wc, FILE* stream);
wint_t putwchar(wchar_t wc);
int swprintf(wchar_t* restrict ws, size_t max_chars, const wchar_t* restrict format, ...);
int swscanf(const wchar_t* restrict ws, const wchar_t* restrict format, ...);
wint_t ungetwc(wint_t wc, FILE* stream);
int vfwprintf(FILE* restrict stream, const wchar_t* restrict format, va_list arg);
int vwprintf(const wchar_t* format, va_list arg);
int wprintf(const wchar_t* format, ...);
int wscanf(const wchar_t* format, ...);

/* String conversion (mirroring stdlib.h) */
double wcstod(const wchar_t* restrict ws, wchar_t** restrict endptr);
long int wcstol(const wchar_t* restrict ws, wchar_t** restrict endptr, int base);
unsigned long int wcstoul(const wchar_t* restrict ws, wchar_t** restrict endptr, int base);
wint_t btowc(int c);
size_t mbrlen(const char* restrict mbc, size_t max_bytes, mbstate_t* restrict state);
size_t mbrtowc(wchar_t* restrict wc, const char* mbc, size_t max_bytes, mbstate_t* restrict state);
int mbsinit(const mbstate_t* state);
size_t mbsrtowcs(wchar_t* restrict dest, const char** restrict src, size_t max_chars, mbstate_t* restrict state);
size_t wcrtomb(char* restrict mbc, wchar_t wc, mbstate_t* restrict state);
int wctob(wint_t wc);
size_t wcsrtombs(char* restrict dest, const wchar_t** restrict src, size_t max_bytes, mbstate_t* restrict state);

/* String manipulation (mirroring string.h) */
wchar_t* wcscat(wchar_t* restrict dest, const wchar_t* restrict src);
wchar_t* wcschr(const wchar_t* ws, wchar_t wc);
int wcscmp(const wchar_t* ws1, const wchar_t* ws2);
int wcscoll(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcscpy(wchar_t* restrict dest, const wchar_t* restrict src);
size_t wcscspn(const wchar_t* ws1, const wchar_t* ws2);
size_t wcslen(const wchar_t* ws);
wchar_t* wcsncat(wchar_t* restrict dest, const wchar_t* restrict src, size_t max_chars);
int wcsncmp(const wchar_t* ws1, const wchar_t* ws2, size_t max_chars);
wchar_t* wcsncpy(wchar_t* restrict dest, const wchar_t* restrict src, size_t max_chars);
wchar_t* wcspbrk(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcsrchr(const wchar_t* ws, wchar_t wc);
size_t wcsspn(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcsstr(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcstok(wchar_t* restrict ws, const wchar_t* restrict delimiters, wchar_t** restrict rest);
size_t wcsxfrm(wchar_t* restrict dest, const wchar_t* restrict src, size_t max_chars);
wchar_t* wmemchr(const wchar_t* ptr, wchar_t wc, size_t num);
int wmemcmp(const wchar_t* ptr1, const wchar_t* ptr2, size_t num);
wchar_t* wmemcpy(wchar_t* restrict dest, const wchar_t* restrict src, size_t num);
wchar_t* wmemmove(wchar_t* dest, const wchar_t* src, size_t num);
wchar_t* wmemset(wchar_t* dest, wchar_t wc, size_t num);

/* Time (mirroring time.h) */
size_t wcsftime(wchar_t* dest, size_t max_chars, const wchar_t* format, const struct tm* timeptr);

#ifdef __cplusplus
#undef restrict
}
#endif

#endif /* _WCHAR_H */
