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

/* Character classification functions */
/* TODO
int isalnum(int c);
int isalpha(int c);
int isblank(int c);
int iscntrl(int c);
int isdigit(int c);
int isgraph(int c); */

int islower(int c) {
    /* FIXME: This is correct for the default "C" locale, but other locales may differ. */
    return c >= 'a' && c <= 'z';
}

/* TODO
int isprint(int c);
int ispunct(int c); */

int isspace(int c) {
    /* FIXME: This is correct for the default "C" locale, but other locales may differ. */
    /* ASCII codes 0x20 and 0x09 through 0x0d */
    return c == ' ' || (c >= '\t' && c <= '\r');
}

/* TODO
int isupper(int c);
int isxdigit(int c); */


/* Character conversion functions */
int tolower(int c) {
    /* FIXME: This is correct for the default "C" locale, but other locales may differ. */
    if (islower(c)) {
        return c + ('a' - 'A');
    }
    return c;
}

int toupper(int c) {
    /* FIXME: This is correct for the default "C" locale, but other locales may differ. */
    if (islower(c)) {
        return c - ('a' - 'A');
    }
    return c;
}
