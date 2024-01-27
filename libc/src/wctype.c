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

#include <locale.h>
#include <wctype.h>

/* Every function in wctype.h not mentioned here is defined in locale.c. */

/* Wide character classification */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswupper.html */
int iswupper(wint_t wc) {
    return iswupper_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswlower.html */
int iswlower(wint_t wc) {
    return iswlower_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswalpha.html */
int iswalpha(wint_t wc) {
    return iswalpha_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswdigit.html */
int iswdigit(wint_t wc) {
    return iswdigit_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswxdigit.html */
int iswxdigit(wint_t wc) {
    return iswxdigit_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswalnum.html */
int iswalnum(wint_t wc) {
    return iswalnum_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswpunct.html */
int iswpunct(wint_t wc) {
    return iswpunct_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswblank.html */
int iswblank(wint_t wc) {
    return iswblank_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswspace.html */
int iswspace(wint_t wc) {
    return iswspace_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswgraph.html */
int iswgraph(wint_t wc) {
    return iswgraph_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswprint.html */
int iswprint(wint_t wc) {
    return iswprint_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswcntrl.html */
int iswcntrl(wint_t wc) {
    return iswcntrl_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/wctype.html */
wctype_t wctype(const char* charclass) {
    return wctype_l(charclass, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswctype.html */
int iswctype(wint_t wc, wctype_t charclass) {
    return iswctype_l(wc, charclass, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iswctype_l.html */
int iswctype_l(wint_t wc, wctype_t charclass, locale_t locale) {
    if (!charclass) return 0;
    return (*charclass)(wc, locale);
}

/* Wide character conversion */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/towupper.html */
wint_t towupper(wint_t wc) {
    return towupper_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/towlower.html */
wint_t towlower(wint_t wc) {
    return towlower_l(wc, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/wctrans.html */
wctrans_t wctrans(const char* mapping) {
    return wctrans_l(mapping, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/towctrans.html */
wint_t towctrans(wint_t wc, wctrans_t mapping) {
    return towctrans_l(wc, mapping, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/towctrans_l.html */
wint_t towctrans_l(wint_t wc, wctrans_t mapping, locale_t locale) {
    if (!mapping) return wc;
    return (*mapping)(wc, locale);
}
