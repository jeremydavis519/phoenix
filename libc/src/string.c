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
/* TODO
void* memcpy(void* dest, const void* src, size_t num);
void* memmove(void* dest, const void* src, size_t num);
char* strcpy(char* dest, const char* src);
char* strncpy(char* dest, const char* src, size_t num); */


/* Concatenation */
/* TODO
char* strcat(char* dest, const char* src);
char* strncat(char* dest, const char* src, size_t num); */


/* Comparison */
/* TODO
int memcmp(const void* ptr1, const void* ptr2, size_t num); */

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
int strncmp(const char* s1, const char* s2, size_t num);
int strxfrm(char* dest, const char* src, size_t num); */


/* Searching */
/* TODO
void* memchr(const void* ptr, int value, size_t num);
char* strchr(const char* s, int c);
size_t strcspn(const char* s1, const char* s2);
char* strpbrk(const char* s1, const char* s2);
char* strrchr(const char* s, int c);
size_t strspn(const char* s1, const char* s2);
char* strstr(const char* s1, const char* s2);
char* strtok(char* s, const char* delimiters); */


/* Other */
/* TODO
void* memset(void* ptr, int value, size_t num);
char* strerror(int errnum); */

size_t strlen(const char* s) {
    size_t len = 0;
    while (*s++) {
        ++len;
    }
    return len;
}
