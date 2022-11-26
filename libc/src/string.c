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

#include <stddef.h>
#include <string.h>

/* Copying */
void* memcpy(void* restrict dest, const void* restrict src, size_t count) {
    unsigned char* sdest = dest;
    const unsigned char* ssrc = src;
    while (count--) {
        *sdest++ = *ssrc++;
    }
    return dest;
}

void* memmove(void* dest, const void* src, size_t count) {
    unsigned char* sdest = dest;
    const unsigned char* ssrc = src;
    if (dest < src) {
        while (count--) {
            *sdest++ = *ssrc++;
        }
    } else {
        // Avoid overwriting not-yet-used src bytes by copying backwards.
        sdest += count;
        ssrc += count;
        while (count--) {
            *--sdest = *--ssrc;
        }
    }
    return dest;
}

char* strcpy(char* restrict dest, const char* restrict src) {
    char c;
    while ((c = *src++)) {
        *dest++ = c;
    }
    *dest = '\0';
    return dest;
}

char* strncpy(char* restrict dest, const char* restrict src, size_t count) {
    char* sdest = dest;
    while (count--) {
        if (!(*sdest++ = *src++)) {
            break;
        }
    }

    // The rest of the array needs to be padded with null characters.
    memset(sdest, '\0', count);

    return dest;
}


/* Concatenation */
/* TODO
char* strcat(char* dest, const char* src);
char* strncat(char* dest, const char* src, size_t count); */


/* Comparison */
/* TODO
int memcmp(const void* ptr1, const void* ptr2, size_t count); */

int strcmp(const char* s1, const char* s2) {
    while (*s1 && *s2) {
        int result = (int)*s1++ - (int)*s2++;
        if (result) {
            return result;
        }
    }
    return (int)*s1 - (int)*s2;
}

/* TODO
int strcoll(const char* s1, const char* s2);
int strncmp(const char* s1, const char* s2, size_t count);
int strxfrm(char* restrict dest, const char* restrict src, size_t count); */


/* Searching */
/* TODO
void* memchr(const void* ptr, int value, size_t count);
char* strchr(const char* s, int c);
size_t strcspn(const char* s1, const char* s2);
char* strpbrk(const char* s1, const char* s2);
char* strrchr(const char* s, int c);
size_t strspn(const char* s1, const char* s2);
char* strstr(const char* s1, const char* s2);
char* strtok(char* restrict s, const char* restrict delimiters); */


/* Other */
void* memset(void* dest, int ch, size_t count) {
    unsigned char* sdest = dest;
    while (count--) {
        *sdest++ = ch;
    }
    return dest;
}

/* TODO
char* strerror(int errnum); */

size_t strlen(const char* s) {
    size_t len = 0;
    while (*s++) {
        ++len;
    }
    return len;
}
