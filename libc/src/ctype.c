/* Copyright (c) 2021-2024 Jeremy Davis (jeremydavis519@gmail.com)
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

#include <ctype.h>
#include <locale.h>

/* Character classification functions */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isalnum.html */
int isalnum(int c) {
    return isalnum_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isalpha.html */
int isalpha(int c) {
    return isalpha_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isascii.html */
int isascii(int c) {
    return c >= 0 && c <= 0x7f;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isblank.html */
int isblank(int c) {
    return isblank_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iscntrl.html */
int iscntrl(int c) {
    return iscntrl_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isdigit.html */
int isdigit(int c) {
    return isdigit_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isgraph.html */
int isgraph(int c) {
    return isgraph_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/islower.html */
int islower(int c) {
    return islower_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isprint.html */
int isprint(int c) {
    return isprint_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ispunct.html */
int ispunct(int c) {
    return ispunct_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isspace.html */
int isspace(int c) {
    return isspace_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isupper.html */
int isupper(int c) {
    return isupper_l(c, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isxdigit.html */
int isxdigit(int c) {
    return isxdigit_l(c, uselocale((locale_t)0));
}


/* Character conversion functions */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/toascii.html */
int toascii(int c) {
    return c & 0x7f;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/tolower.html */
#undef tolower
int tolower(int c) {
    return tolower_l(c, uselocale((locale_t)0));
}

/* to_lower_l defined in locale.c */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/toupper.html */
#undef toupper
int toupper(int c) {
    return toupper_l(c, uselocale((locale_t)0));
}

/* to_upper_l defined in locale.c */
