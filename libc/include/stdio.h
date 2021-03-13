/* Copyright (c) 2019-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

/* This file defines the C standard library's I/O functions and types for applications written for
 * Phoenix. Since everything in here is standard, see http://www.cplusplus.com/reference/cstdio/
 * for docs. */

#ifndef _STDIO_H
#define _STDIO_H

#include <stddef.h>
#include <stdarg.h>

#define BUFSIZ 8192
#define EOF -1
#define FILENAME_MAX 4096
#define FOPEN_MAX 16
#define L_tmpnam 10 /* format: /tmp~[0-9a-z]{6}/ */
#define TMP_MAX 0xffffffff

#define _IOFBF 2 /* Full-buffering mode */
#define _IOLBF 1 /* Line-buffering mode */
#define _IONBF 0 /* Non-buffering mode */

#define SEEK_SET 0 /* Origin at beginning of file */
#define SEEK_CUR 1 /* Origin at current position */
#define SEEK_END 2 /* Origin at end of file */

#ifdef __cplusplus
extern "C" {
#endif

typedef size_t FILE;
typedef size_t fpos_t;

FILE* stdin;
FILE* stdout;
FILE* stderr;

/* Operations on files */
int remove(const char* filename);
int rename(const char* oldname, const char* newname);
FILE* tmpfile(void);
char* tmpnam(char* str);

/* File access */
int fclose(FILE* stream);
int fflush(FILE* stream);
FILE* fopen(const char* filename, const char* mode);
FILE* freopen(const char* filename, const char* mode, FILE* stream);
void setbuf(FILE* stream, char* buffer);
int setvbuf(FILE* stream, char* buffer, int mode, size_t size);

/* Formatted input/output */
int fprintf(FILE* stream, const char* format, ...);
int fscanf(FILE* stream, const char* format, ...);
int printf(const char* format, ...);
int scanf(const char* format, ...);
int snprintf(char* s, size_t n, const char* format, ...);
int sprintf(char* s, const char* format, ...);
int sscanf(const char* s, const char* format, ...);
int vfprintf(FILE* stream, const char* format, va_list arg);
int vfscanf(FILE* stream, const char* format, va_list arg);
int vprintf(const char* format, va_list arg);
int vscanf(const char* format, va_list arg);
int vsnprintf(char* s, size_t n, const char* format, va_list arg);
int vsprintf(char* s, const char* format, va_list arg);
int vsscanf(const char* s, const char* format, va_list arg);

/* Character input/output */
int fgetc(FILE* stream);
char* fgets(char* str, int num, FILE* stream);
int fputc(int character, FILE* stream);
int fputs(const char* str, FILE* stream);
int getc(FILE* stream);
int getchar(void);
/* char* gets(char* s) -- Removed from the C standard as of 2011 (prone to buffer overflows) */
int putc(int character, FILE* stream);
int putchar(int character);
int puts(const char* str);
int ungetc(int character, FILE* stream);

/* Direct input/output */
size_t fread(void* ptr, size_t size, size_t count, FILE* stream);
size_t fwrite(const void* ptr, size_t size, size_t count, FILE* stream);

/* File positioning */
int fgetpos(FILE* stream, fpos_t* pos);
int fseek(FILE* stream, long int offset, int origin);
long int ftell(FILE* stream);
void rewind(FILE* stream);

/* Error-handling */
void clearerr(FILE* stream);
int feof(FILE* stream);
int ferror(FILE* stream);
void perror(FILE* stream);

#ifdef __cplusplus
}
#endif

#endif /* _STDIO_H */
