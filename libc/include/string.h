/* Copyright (c) 2021-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

#ifndef __PHOENIX_STRING_H
#define __PHOENIX_STRING_H

#include <locale.h>

#ifdef __cplusplus
extern "C" {
#endif

#if !defined(__cplusplus) && __STDC_VERSION__ >= 199901L
#define _PHOENIX_restrict restrict
#else
#define _PHOENIX_restrict
#endif /* __cplusplus and __STDC_VERSION__ */

/* Copying */
void* memccpy(void* _PHOENIX_restrict dest, const void* _PHOENIX_restrict src, size_t count);
void* memcpy(void* _PHOENIX_restrict dest, const void* _PHOENIX_restrict src, size_t count);
void* memmove(void* dest, const void* src, size_t num);
char* strcpy(char* _PHOENIX_restrict dest, const char* _PHOENIX_restrict src);
char* strncpy(char* _PHOENIX_restrict dest, const char* _PHOENIX_restrict src, size_t count);
char* stpcpy(char* _PHOENIX_restrict dest, const char* _PHOENIX_restrict src);
char* stpncpy(char* _PHOENIX_restrict dest, const char* _PHOENIX_restrict src, size_t count);
char* strdup(const char* src);
char* strndup(const char* src, size_t count);

/* Concatenation */
char* strcat(char* _PHOENIX_restrict dest, const char* _PHOENIX_restrict src);
char* strncat(char* _PHOENIX_restrict dest, const char* _PHOENIX_restrict src, size_t count);

/* Comparison */
int memcmp(const void* ptr1, const void* ptr2, size_t count);
int strcmp(const char* s1, const char* s2);
int strcoll(const char* s1, const char* s2);
int strcoll_l(const char* s1, const char* s2, locale_t locale);
int strncmp(const char* s1, const char* s2, size_t count);
size_t strxfrm(char* _PHOENIX_restrict dest, const char* _PHOENIX_restrict src, size_t count);
size_t strxfrm_l(char* _PHOENIX_restrict dest, const char* _PHOENIX_restrict src, size_t count, locale_t locale);

/* Searching */
void* memchr(const void* ptr, int value, size_t count);
char* strchr(const char* s, int c);
size_t strcspn(const char* s1, const char* s2);
char* strpbrk(const char* s1, const char* s2);
char* strrchr(const char* s, int c);
size_t strspn(const char* s1, const char* s2);
char* strstr(const char* s1, const char* s2);
char* strtok(char* _PHOENIX_restrict s, const char* _PHOENIX_restrict delimiters);
char* strtok_r(char* _PHOENIX_restrict s, const char* _PHOENIX_restrict delimiters, char** _PHOENIX_restrict state);

/* Other */
void* memset(void* dest, int ch, size_t count);
char* strerror(int errnum);
char* strerror_l(int errnum, locale_t locale);
int strerror_r(int errnum, char* strerrbuf, size_t buflen);
size_t strlen(const char* s);
size_t strnlen(const char* s, size_t max);
char* strsignal(int signal);

#ifdef __cplusplus
}
#endif

#endif /* __PHOENIX_STRING_H */
