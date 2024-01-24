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

#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <wchar.h>
#include "stdiotyp.h"

#define EFAIL(e) do { errno = (e); goto fail; } while (0)
#define UCHAR_TO_CHAR(c) ((int)(c) > CHAR_MAX ? (char)((int)(c) - ((int)UCHAR_MAX + 1)) : (char)(c))

/* Input/output (mirroring stdio.h) */
/* TODO
FILE* open_wmemstream(wchar_t** bufp, size_t* sizep);
wint_t fgetwc(FILE* stream);
wint_t getwc(FILE* stream);
wint_t getwchar(void);
wchar_t* fgetws(wchar_t* _PHOENIX_restrict ws, int max_chars, FILE* _PHOENIX_restrict stream); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fputwc.html */
wint_t fputwc(wchar_t wc, FILE* stream) {
    flockfile(stream);
    wint_t result = _PHOENIX_fputwc_unlocked(wc, stream);
    funlockfile(stream);
    return result;
}

wint_t _PHOENIX_fputwc_unlocked(wchar_t wc, FILE* stream) {
    char buffer[MB_LEN_MAX];
    size_t buffer_len = wcrtomb(buffer, wc, &stream->position.mb_parse_state);
    if (buffer_len == (size_t)-1) {
        stream->error = -1;
        return WEOF;
    }
    if (_PHOENIX_fwrite_unlocked(buffer, buffer_len, 1, stream) != 1) return WEOF;
    return wc;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/putwc.html */
wint_t putwc(wchar_t wc, FILE* stream) {
    return fputwc(wc, stream);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/putwchar.html */
wint_t putwchar(wchar_t wc) {
    return putwc(wc, stdout);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fputws.html */
int fputws(const wchar_t* _PHOENIX_restrict ws, FILE* _PHOENIX_restrict stream) {
    flockfile(stream);
    wchar_t wc;
    while ((wc = *ws++)) {
        if (_PHOENIX_fputwc_unlocked(wc, stream) == WEOF) return -1;
    }
    funlockfile(stream);
    return 0;
}

/* TODO
int fwprintf(FILE* _PHOENIX_restrict stream, const wchar_t* _PHOENIX_restrict format, ...);
int wprintf(const wchar_t* format, ...);
int vfwprintf(FILE* _PHOENIX_restrict stream, const wchar_t* _PHOENIX_restrict format, va_list arg);
int vwprintf(const wchar_t* format, va_list arg);
int swprintf(wchar_t* _PHOENIX_restrict ws, size_t max_chars, const wchar_t* _PHOENIX_restrict format, ...);
int fwscanf(FILE* _PHOENIX_restrict stream, const wchar_t* _PHOENIX_restrict format, ...);
int wscanf(const wchar_t* format, ...);
int swscanf(const wchar_t* _PHOENIX_restrict ws, const wchar_t* _PHOENIX_restrict format, ...);
wint_t ungetwc(wint_t wc, FILE* stream);
int fwide(FILE* stream, int mode); */

/* Character conversion (mirroring ctype.h) */
/* TODO
wint_t towlower(wint_t wc);
wint_t towupper(wint_t wc); */

/* String conversion (mirroring stdlib.h) */
/* TODO
double wcstod(const wchar_t* _PHOENIX_restrict ws, wchar_t** _PHOENIX_restrict endptr);
long int wcstol(const wchar_t* _PHOENIX_restrict ws, wchar_t** _PHOENIX_restrict endptr, int base);
unsigned long int wcstoul(const wchar_t* _PHOENIX_restrict ws, wchar_t** _PHOENIX_restrict endptr, int base);
wint_t btowc(int c);
size_t mbrlen(const char* _PHOENIX_restrict mbc, size_t max_bytes, mbstate_t* _PHOENIX_restrict state);
size_t mbrtowc(wchar_t* _PHOENIX_restrict wc, const char* mbc, size_t max_bytes, mbstate_t* _PHOENIX_restrict state);
int mbsinit(const mbstate_t* state);
size_t mbsnrtowcs(wchar_t* _PHOENIX_restrict dest, const char** _PHOENIX_restrict src, size_t max_bytes, size_t max_chars, mbstate_t* _PHOENIX_restrict state);
size_t mbsrtowcs(wchar_t* _PHOENIX_restrict dest, const char** _PHOENIX_restrict src, size_t max_chars, mbstate_t* _PHOENIX_restrict state); */

size_t wcrtomb(char* _PHOENIX_restrict mbc, wchar_t wc, mbstate_t* _PHOENIX_restrict state) {
    static char buf[MB_LEN_MAX];

    if (!mbc) return wcrtomb(buf, L'\0', state);

    /* FIXME: For non-POSIX locales (e.g. ones that support UTF-8), encode the wide character as a byte stream correctly. */

    if (
#if WCHAR_MIN < 0
        wc < 0 ||
#endif
        wc > 255) EFAIL(EILSEQ);

    *mbc = UCHAR_TO_CHAR(wc);
    return 1;

fail:
    return (size_t)-1;
}

/* TODO
int wctob(wint_t wc);
size_t wcsrtombs(char* _PHOENIX_restrict dest, const wchar_t** _PHOENIX_restrict src, size_t max_bytes, mbstate_t* _PHOENIX_restrict state);
size_t wcsnrtombs(char* _PHOENIX_restrict dest, const wchar_t** _PHOENIX_restrict src, size_t max_chars, size_t max_bytes, mbstate_t* _PHOENIX_restrict state); */

/* String manipulation (mirroring string.h) */
/* TODO
wchar_t* wcscat(wchar_t* _PHOENIX_restrict dest, const wchar_t* _PHOENIX_restrict src);
wchar_t* wcschr(const wchar_t* ws, wchar_t wc);
int wcscmp(const wchar_t* ws1, const wchar_t* ws2);
int wcscasecmp(const wchar_t* ws1, const wchar_t* ws2);
int wcscasecmp_l(const wchar_t* ws1, const wchar_t* ws2, locale_t locale);
int wcscoll(const wchar_t* ws1, const wchar_t* ws2);
int wcscoll_l(const wchar_t* ws1, const wchar_t* ws2, locale_t locale);
wchar_t* wcscpy(wchar_t* _PHOENIX_restrict dest, const wchar_t* _PHOENIX_restrict src);
wchar_t* wcpcpy(wchar_t* _PHOENIX_restrict dest, const wchar_t* _PHOENIX_restrict src);
wchar_t* wcsdup(const wchar_t* ws);
size_t wcscspn(const wchar_t* ws1, const wchar_t* ws2);
size_t wcslen(const wchar_t* ws);
wchar_t* wcsncat(wchar_t* _PHOENIX_restrict dest, const wchar_t* _PHOENIX_restrict src, size_t max_chars);
int wcsncmp(const wchar_t* ws1, const wchar_t* ws2, size_t max_chars);
int wcsncasecmp(const wchar_t* ws1, const wchar_t* ws2, size_t max_chars);
int wcsncasecmp_l(const wchar_t* ws1, const wchar_t* ws2, size_t max_chars, locale_t locale);
wchar_t* wcsncpy(wchar_t* _PHOENIX_restrict dest, const wchar_t* _PHOENIX_restrict src, size_t max_chars);
wchar_t* wcpncpy(wchar_t _PHOENIX_restrict dest, const wchar_t* _PHOENIX_restrict src, size_t max_chars);
wchar_t* wcspbrk(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcsrchr(const wchar_t* ws, wchar_t wc);
size_t wcsspn(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcsstr(const wchar_t* ws1, const wchar_t* ws2);
wchar_t* wcstok(wchar_t* _PHOENIX_restrict ws, const wchar_t* _PHOENIX_restrict delimiters, wchar_t** _PHOENIX_restrict rest);
size_t wcsxfrm(wchar_t* _PHOENIX_restrict dest, const wchar_t* _PHOENIX_restrict src, size_t max_chars);
size_t wcsxfrm_l(wchar_t* _PHOENIX_restrict dest, const wchar_t* _PHOENIX_restrict src, size_t max_chars, locale_t locale);
wchar_t* wmemchr(const wchar_t* ptr, wchar_t wc, size_t num);
int wmemcmp(const wchar_t* ptr1, const wchar_t* ptr2, size_t num);
wchar_t* wmemcpy(wchar_t* _PHOENIX_restrict dest, const wchar_t* _PHOENIX_restrict src, size_t num);
wchar_t* wmemmove(wchar_t* dest, const wchar_t* src, size_t num);
wchar_t* wmemset(wchar_t* dest, wchar_t wc, size_t num); */

/* Time (mirroring time.h) */
size_t wcsftime(wchar_t* dest, size_t max_chars, const wchar_t* format, const struct tm* timeptr);

/* Display */
/* TODO
int wcwidth(wchar_t wc);
int wcswidth(const wchar_t* ws, size_t n); */
