/* Copyright (c) 2019-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

#ifndef __PHOENIX_STDIO_H
#define __PHOENIX_STDIO_H

#include <stdarg.h>
#include <stddef.h>

#define BUFSIZ 8192
#define EOF -1
#define FILENAME_MAX 4096
#define FOPEN_MAX 16
#define L_tmpnam 9 /* format: /t~[0-9a-z]{6}/ */
#define TMP_MAX 0x7fffffff

#define _IOFBF 2 /* Full-buffering mode */
#define _IOLBF 1 /* Line-buffering mode */
#define _IONBF 0 /* Non-buffering mode */

#define SEEK_SET 0 /* Origin at beginning of file */
#define SEEK_CUR 1 /* Origin at current position */
#define SEEK_END 2 /* Origin at end of file */

#ifdef __cplusplus
extern "C" {
#define restrict
#endif

typedef struct FILE   FILE;
typedef struct fpos_t fpos_t;

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
FILE* fopen(const char* restrict filename, const char* restrict mode);
FILE* freopen(const char* restrict filename, const char* restrict mode, FILE* restrict stream);
void setbuf(FILE* restrict stream, char* restrict buffer);
int setvbuf(FILE* restrict stream, char* restrict buffer, int mode, size_t size);

/* Formatted input/output */
int fprintf(FILE* restrict stream, const char* restrict format, ...);
int fscanf(FILE* restrict stream, const char* restrict format, ...);
int printf(const char* format, ...);
int scanf(const char* format, ...);
int snprintf(char* restrict s, size_t n, const char* restrict format, ...);
int sprintf(char* restrict s, const char* restrict format, ...);
int sscanf(const char* restrict s, const char* restrict format, ...);
int vfprintf(FILE* restrict stream, const char* restrict format, va_list args);
int vfscanf(FILE* restrict stream, const char* restrict format, va_list args);
int vprintf(const char* format, va_list args);
int vscanf(const char* format, va_list args);
int vsnprintf(char* restrict s, size_t n, const char* restrict format, va_list args);
int vsprintf(char* restrict s, const char* restrict format, va_list args);
int vsscanf(const char* restrict s, const char* restrict format, va_list args);

/* Character input/output */
int fgetc(FILE* stream);
#define getc fgetc
char* fgets(char* restrict str, int num, FILE* restrict stream);
int fputc(int character, FILE* stream);
#define putc fputc
int fputs(const char* restrict str, FILE* restrict stream);
int getc(FILE* stream);
#define getchar() fgetc(stdin)
/* char* gets(char* s) -- Removed from the C standard as of 2011 (prone to buffer overflows) */
#define putchar(character) fputc(character, stdout)
int puts(const char* str);
int ungetc(int character, FILE* stream);

/* Direct input/output */
size_t fread(void* restrict ptr, size_t size, size_t count, FILE* restrict stream);
size_t fwrite(const void* restrict ptr, size_t size, size_t count, FILE* restrict stream);

/* File positioning */
int fgetpos(FILE* restrict stream, fpos_t* restrict pos);
int fsetpos(FILE* stream, const fpos_t* pos);
int fseek(FILE* stream, long int offset, int origin);
long int ftell(FILE* stream);
void rewind(FILE* stream);

/* Error-handling */
void clearerr(FILE* stream);
int feof(FILE* stream);
int ferror(FILE* stream);
void perror(const char* s);

#ifdef __cplusplus
#undef restrict
}
#endif

#endif /* __PHOENIX_STDIO_H */
