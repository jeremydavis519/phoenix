/* Copyright (c) 2022 Jeremy Davis (jeremydavis519@gmail.com)
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

/* Locale information as specified by POSIX
   https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/locale.h.html */

#ifndef __PHOENIX_LOCALE_H
#define __PHOENIX_LOCALE_H

#include <stddef.h>
#include <stdint.h>

struct lconv {
    char* currency_symbol;
    char* decimal_point;
    char  frac_digits;
    char* grouping;
    char* int_curr_symbol;
    char  int_frac_digits;
    char  int_n_cs_precedes;
    char  int_n_sep_by_space;
    char  int_n_sign_posn;
    char  int_p_cs_precedes;
    char  int_p_sep_by_space;
    char  int_p_sign_posn;
    char* mon_decimal_point;
    char* mon_grouping;
    char* mon_thousands_sep;
    char* negative_sign;
    char  n_cs_precedes;
    char  n_sep_by_space;
    char  n_sign_posn;
    char* positive_sign;
    char  p_cs_precedes;
    char  p_sep_by_space;
    char  p_sign_posn;
    char* thousands_sep;
};

#define LC_COLLATE  0
#define LC_CTYPE    1
#define LC_MESSAGES 2
#define LC_MONETARY 3
#define LC_NUMERIC  4
#define LC_TIME     5
#define LC_ALL      -1

#define LC_COLLATE_MASK     0x01
#define LC_CTYPE_MASK       0x02
#define LC_MESSAGES_MASK    0x04
#define LC_MONETARY_MASK    0x08
#define LC_NUMERIC_MASK     0x10
#define LC_TIME_MASK        0x20
#define LC_ALL_MASK         (int)UINT_MAX

typedef void* locale_t;

const locale_t LC_GLOBAL_LOCALE = ""; /* Any non-null pointer will do as long as it can't ever point to a real locale. */

locale_t        duplocale(locale_t);
void            freelocale(locale_t);
struct lconv*   localeconv(void);
locale_t        newlocale(int category_mask, const char* locale, locale_t base);
char*           setlocale(int category, const char* locale);
locale_t        uselocale(locale_t newloc);

#endif /* __PHOENIX_LOCALE_H */
