/* Copyright (c) 2019-2024 Jeremy Davis (jeremydavis519@gmail.com)
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

/* This file defines the C standard library's functions for classifying and transforming
 * individual characters. Since everything in here is standard, see
 * http://www.cplusplus.com/reference/cctype/ for docs. */

#ifndef __PHOENIX_CTYPE_H
#define __PHOENIX_CTYPE_H

#include <locale.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Character classification functions */
int isalnum(int c);
int isalnum_l(int c, locale_t locale);
int isalpha(int c);
int isalpha_l(int c, locale_t locale);
int isascii(int c);
int isblank(int c);
int isblank_l(int c, locale_t locale);
int iscntrl(int c);
int iscntrl_l(int c, locale_t locale);
int isdigit(int c);
int isdigit_l(int c, locale_t locale);
int isgraph(int c);
int isgraph_l(int c, locale_t locale);
int islower(int c);
int islower_l(int c, locale_t locale);
int isprint(int c);
int isprint_l(int c, locale_t locale);
int ispunct(int c);
int ispunct_l(int c, locale_t locale);
int isspace(int c);
int isspace_l(int c, locale_t locale);
int isupper(int c);
int isupper_l(int c, locale_t locale);
int isxdigit(int c);
int isxdigit_l(int c, locale_t locale);

/* Character conversion functions */
int toascii(int c);
int tolower(int c);
#define tolower(c) (_PHOENIX_tolower_l((c), _PHOENIX_uselocale((locale_t)0)))
#define _tolower(c) (_PHOENIX_tolower_l((c), _PHOENIX_uselocale((locale_t)0)))
int tolower_l(int c, locale_t locale);
#define tolower_l(c, locale) (_PHOENIX_tolower_l((c), (locale)))
int _PHOENIX_tolower_l(int c, locale_t locale);
int toupper(int c);
#define toupper(c) (_PHOENIX_toupper_l((c), _PHOENIX_uselocale((locale_t)0)))
#define _toupper(c) (_PHOENIX_toupper_l((c), _PHOENIX_uselocale((locale_t)0)))
int toupper_l(int c, locale_t locale);
#define toupper_l(c, locale) (_PHOENIX_toupper_l((c), (locale)))
int _PHOENIX_toupper_l(int c, locale_t locale);

#ifdef __cplusplus
}
#endif

#endif /* __PHOENIX_CTYPE_H */
