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
#include <sys/types.h>

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
#endif

#if !defined(__cplusplus) && __STDC_VERSION__ >= 199901L
#define _PHOENIX_restrict restrict
#else
#define _PHOENIX_restrict
#endif /* __cplusplus and __STDC_VERSION__ */

typedef struct FILE   FILE;
typedef struct fpos_t fpos_t;

extern FILE* stdin;
extern FILE* stdout;
extern FILE* stderr;

/* Operations on files */
int remove(const char* path);
int rename(const char* oldname, const char* newname);
FILE* tmpfile(void);
char* tmpnam(char* str);

/* File access */
int fclose(FILE* stream);
int fflush(FILE* stream);
FILE* fopen(const char* _PHOENIX_restrict path, const char* _PHOENIX_restrict mode);
FILE* freopen(const char* _PHOENIX_restrict path, const char* _PHOENIX_restrict mode, FILE* _PHOENIX_restrict stream);
void setbuf(FILE* _PHOENIX_restrict stream, char* _PHOENIX_restrict buffer);
int setvbuf(FILE* _PHOENIX_restrict stream, char* _PHOENIX_restrict buffer, int mode, size_t size);

/* Formatted input/output */
int fprintf(FILE* _PHOENIX_restrict stream, const char* _PHOENIX_restrict format, ...);
int fscanf(FILE* _PHOENIX_restrict stream, const char* _PHOENIX_restrict format, ...);
int printf(const char* format, ...);
int scanf(const char* format, ...);
int snprintf(char* _PHOENIX_restrict s, size_t n, const char* _PHOENIX_restrict format, ...);
int sprintf(char* _PHOENIX_restrict s, const char* _PHOENIX_restrict format, ...);
int sscanf(const char* _PHOENIX_restrict s, const char* _PHOENIX_restrict format, ...);
int vfprintf(FILE* _PHOENIX_restrict stream, const char* _PHOENIX_restrict format, va_list args);
int vfscanf(FILE* _PHOENIX_restrict stream, const char* _PHOENIX_restrict format, va_list args);
int vprintf(const char* format, va_list args);
int vscanf(const char* format, va_list args);
int vsnprintf(char* _PHOENIX_restrict s, size_t n, const char* _PHOENIX_restrict format, va_list args);
int vsprintf(char* _PHOENIX_restrict s, const char* _PHOENIX_restrict format, va_list args);
int vsscanf(const char* _PHOENIX_restrict s, const char* _PHOENIX_restrict format, va_list args);

/* Character input/output */
int fgetc(FILE* stream);
int getc(FILE* stream);
char* fgets(char* _PHOENIX_restrict str, int num, FILE* _PHOENIX_restrict stream);
int fputc(int ch, FILE* stream);
int putc(int ch, FILE* stream);
int fputs(const char* _PHOENIX_restrict str, FILE* _PHOENIX_restrict stream);
int getc(FILE* stream);
int getchar(void);
/* char* gets(char* s) -- Removed from the C standard as of 2011 (prone to buffer overflows) */
int putchar(int ch);
int puts(const char* str);
int ungetc(int ch, FILE* stream);

/* Direct input/output */
size_t fread(void* _PHOENIX_restrict buffer, size_t size, size_t count, FILE* _PHOENIX_restrict stream);
size_t fwrite(const void* _PHOENIX_restrict buffer, size_t size, size_t count, FILE* _PHOENIX_restrict stream);

/* File positioning */
int fgetpos(FILE* _PHOENIX_restrict stream, fpos_t* _PHOENIX_restrict pos);
int fsetpos(FILE* stream, const fpos_t* pos);
int fseek(FILE* stream, long offset, int whence);
int fseeko(FILE* stream, off_t offset, int whence);
long ftell(FILE* stream);
off_t ftello(FILE* stream);
void rewind(FILE* stream);

/* Error-handling */
void clearerr(FILE* stream);
int feof(FILE* stream);
int ferror(FILE* stream);
void perror(const char* s);

#ifdef __cplusplus
}
#endif

#endif /* __PHOENIX_STDIO_H */
