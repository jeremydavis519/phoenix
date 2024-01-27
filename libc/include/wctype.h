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

#ifndef __PHOENIX_WCTYPE_H
#define __PHOENIX_WCTYPE_H

#include <wchar.h>
#include <phoenix/locale_t.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef wint_t (*wctrans_t)(wint_t, locale_t);

#if !defined(__cplusplus) && __STDC_VERSION__ >= 199901L
#define _PHOENIX_restrict restrict
#else
#define _PHOENIX_restrict
#endif /* __cplusplus and __STDC_VERSION__ */

/* Wide character classification */
int iswupper(wint_t wc);
int iswupper_l(wint_t wc, locale_t locale);
int iswlower(wint_t wc);
int iswlower_l(wint_t wc, locale_t locale);
int iswalpha(wint_t wc);
int iswalpha_l(wint_t wc, locale_t locale);
int iswdigit(wint_t wc);
int iswdigit_l(wint_t wc, locale_t locale);
int iswxdigit(wint_t wc);
int iswxdigit_l(wint_t wc, locale_t locale);
int iswalnum(wint_t wc);
int iswalnum_l(wint_t wc, locale_t locale);
int iswpunct(wint_t wc);
int iswpunct_l(wint_t wc, locale_t locale);
int iswblank(wint_t wc);
int iswblank_l(wint_t wc, locale_t locale);
int iswspace(wint_t wc);
int iswspace_l(wint_t wc, locale_t locale);
int iswgraph(wint_t wc);
int iswgraph_l(wint_t wc, locale_t locale);
int iswprint(wint_t wc);
int iswprint_l(wint_t wc, locale_t locale);
int iswcntrl(wint_t wc);
int iswcntrl_l(wint_t wc, locale_t locale);
wctype_t wctype(const char* charclass);
wctype_t wctype_l(const char* charclass, locale_t locale);
int iswctype(wint_t wc, wctype_t charclass);
int iswctype_l(wint_t wc, wctype_t charclass, locale_t locale);

/* Wide character conversion */
wint_t towupper(wint_t wc);
wint_t towupper_l(wint_t wc, locale_t locale);
wint_t towlower(wint_t wc);
wint_t towlower_l(wint_t wc, locale_t locale);
wctrans_t wctrans(const char* trans);
wctrans_t wctrans_l(const char* trans, locale_t locale);
wint_t towctrans(wint_t wc, wctrans_t trans);
wint_t towctrans_l(wint_t wc, wctrans_t trans, locale_t locale);

#ifdef __cplusplus
}
#endif

#endif /* __PHOENIX_WCTYPE_H */
