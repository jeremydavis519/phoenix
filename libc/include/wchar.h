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

#ifndef _WCHAR_H
#define _WCHAR_H

#include <stddef.h>
#include <stdio.h>
#include <time.h>

#ifdef __cplusplus
extern "C" {
#endif

#define WCHAR_MAX 0x10ffff
#define WCHAR_MIN 0
#define WEOF -1

typedef unsigned char mbstate_t;
typedef wchar_t wint_t;

/* Input/output (mirroring stdio.h) */
wint_t fgetwc(FILE* stream);
wchar_t* fgetws(wchar_t* ws, int max_chars, FILE* stream);
wint_t fputwc(wchar_t wc, FILE* stream);
int fputws(const wchar_t* ws, FILE* stream);
int fwide(FILE* stream, int mode);
int fwprintf(FILE* stream, const wchar_t* format, ...);
int fwscanf(FILE* stream, const wchar_t* format, ...);
wint_t getwc(FILE* stream);
wint_t getwchar(void);
wint_t putwc(wchar_t wc, FILE* stream);
wint_t putwchar(wchar_t wc);
int swprintf(wchar_t* ws, size_t max_chars, const wchar_t* format, ...);
int swscanf(const wchar_t* ws, const wchar_t* format, ...);
wint_t ungetwc(wint_t wc, FILE* stream);
int vfwprintf(FILE* stream, const wchar_t* format, va_list arg);
int vwprintf(const wchar_t* format, va_list arg);
int wprintf(const wchar_t* format, ...);
int wscanf(const wchar_t* format, ...);

/* String conversion (mirroring stdlib.h) */
double wcstod(const wchar_t* ws, wchar_t** endptr);
long int wcstol(const wchar_t* ws, wchar_t** endptr, int base);
unsigned long int wcstoul(const wchar_t* ws, wchar_t** endptr, int base);
wint_t btowc(int c);
size_t mbrlen(const char* mbc, size_t max_bytes, mbstate_t* state);
size_t mbrtowc(wchar_t* wc, const char* mbc, size_t max_bytes, mbstate_t* state);
int mbsinit(const mbstate_t* state);
size_t mbsrtowcs(wchar_t* dest, const char** src, size_t max_chars, mbstate_t* state);
size_t wcrtomb(char* mbc, wchar_t wc, mbstate_t* state);
int wctob(wint_t wc);
size_t wcsrtombs(char* dest, const wchar_t** src, size_t max_bytes, mbstate_t* state);

/* String manipulation (mirroring string.h) */
wchar_t* wcscat(wchar_t* dest, const wchar_t* src);
wchar_t* wcschr(const wchar_t* ws, wchar_t wc);
int wcscmp(const wchar_t* ws1, const wchar_t* ws2);
int wcscoll(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcscpy(wchar_t* dest, const wchar_t* src);
size_t wcscspn(const wchar_t* ws1, const wchar_t* ws2);
size_t wcslen(const wchar_t* ws);
wchar_t* wcsncat(wchar_t* dest, const wchar_t* src, size_t max_chars);
int wcsncmp(const wchar_t* ws1, const wchar_t* ws2, size_t max_chars);
wchar_t* wcsncpy(wchar_t* dest, const wchar_t* src, size_t max_chars);
wchar_t* wcspbrk(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcsrchr(const wchar_t* ws, wchar_t wc);
size_t wcsspn(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcsstr(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcstok(wchar_t* ws, const wchar_t* delimiters, wchar_t** rest);
size_t wcsxfrm(wchar_t* dest, const wchar_t* src, size_t max_chars);
wchar_t* wmemchr(const wchar_t* ptr, wchar_t wc, size_t num);
int wmemcmp(const wchar_t* ptr1, const wchar_t* ptr2, size_t num);
wchar_t* wmemcpy(wchar_t* dest, const wchar_t* src, size_t num);
wchar_t* wmemmove(wchar_t* dest, const wchar_t* src, size_t num);
wchar_t* wmemset(wchar_t* dest, wchar_t wc, size_t num);

/* Time (mirroring time.h) */
size_t wcsftime(wchar_t* dest, size_t max_chars, const wchar_t* format, const struct tm* timeptr);

#ifdef __cplusplus
}
#endif

#endif /* _WCHAR_H */
