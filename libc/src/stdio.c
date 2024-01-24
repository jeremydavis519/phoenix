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
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <wchar.h>
#include "stdiotyp.h"

typedef uint32_t FormatSpecFlags;
#define FSF_SIGN      0x00001
#define FSF_RADIX     0x0000e
#define FSF_TEXT_TYPE 0x000f0
#define FSF_ARG_TYPE  0x00f00

#define FSF_SIGNED                 0x000000
#define FSF_UNSIGNED               0x000001

#define FSF_ANY_RADIX              0x000000
#define FSF_DECIMAL                0x000002
#define FSF_OCTAL                  0x000004
#define FSF_HEX_LOWER              0x000006
#define FSF_HEX_UPPER              0x000008

#define FSF_TEXT_INTEGER           0x000000
#define FSF_TEXT_FLOAT_LOWER       0x000010
#define FSF_TEXT_FLOAT_UPPER       0x000020
#define FSF_TEXT_FLOAT_SCI_LOWER   0x000030
#define FSF_TEXT_FLOAT_SCI_UPPER   0x000040
#define FSF_TEXT_FLOAT_FLEX_LOWER  0x000050
#define FSF_TEXT_FLOAT_FLEX_UPPER  0x000060
#define FSF_TEXT_CHAR              0x000070
#define FSF_TEXT_STRING            0x000080
#define FSF_TEXT_POINTER           0x000090
#define FSF_TEXT_SCANSET           0x0000a0
#define FSF_TEXT_COUNT             0x0000b0
#define FSF_TEXT_PERCENT           0x0000c0

#define FSF_ARG_DEFAULT            0x000000
#define FSF_ARG_CHAR               0x000100
#define FSF_ARG_SHORT              0x000200
#define FSF_ARG_LONG               0x000300
#define FSF_ARG_LONG_LONG          0x000400
#define FSF_ARG_INTMAX_T           0x000500
#define FSF_ARG_SIZE_T             0x000600
#define FSF_ARG_PTRDIFF_T          0x000700
#define FSF_ARG_LONG_DOUBLE        0x000800

#define FSF_THOUSANDS              0x001000
#define FSF_JUSTIFY_LEFT           0x002000
#define FSF_FORCE_SIGN             0x004000
#define FSF_SPACE_AS_SIGN          0x008000
#define FSF_DECORATE               0x010000
#define FSF_PAD_WITH_ZERO          0x020000
#define FSF_SCANSET_NEGATED        0x040000
#define FSF_HAS_PRECISION          0x080000
#define FSF_HAS_WIDTH              0x100000
#define FSF_PRECISION_FROM_ARG     0x200000
#define FSF_WIDTH_FROM_ARG         0x400000

typedef struct FormatSpec {
    long            argpos;           /* 0 indicates the next argument */
    FormatSpecFlags flags;
    size_t          precision;
    long            precision_argpos; /* 0 indicates the next argument */
    size_t          width;
    const char*     scanner;
} FormatSpec;

static int parse_format_spec(const char* restrict* restrict format, FormatSpec* restrict spec);
static int parse_scanset(const char* restrict* restrict format, FormatSpec* restrict spec);
static long find_positioned_args(const char* restrict format, va_list args, va_list positioned_args[NL_ARGMAX]);

static FILE files[FOPEN_MAX] = {0};


/* Standard input and output */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/stdin.html */
FILE* stdin  = &files[0];
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/stdout.html */
FILE* stdout = &files[1];
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/stderr.html */
FILE* stderr = &files[2];


#define EFAIL(e) do { errno = (e); goto fail; } while (0)
#define UCHAR_TO_CHAR(c) ((int)(c) > CHAR_MAX ? (char)((int)(c) - ((int)UCHAR_MAX + 1)) : (char)(c))


/* Operations on files */
/* TODO 
int remove(const char* path);
int rename(const char* oldname, const char* newname);
FILE* tmpfile(void);
char* tmpnam(char* str); */


/* File access */
/* TODO 
int fclose(FILE* stream); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fflush.html */
int fflush(FILE* stream) {
    if (!stream) {
        /* FIXME: Flush all open streams. */
        return EOF;
    }

    flockfile(stream);
    int result = _PHOENIX_fflush_unlocked(stream);
    funlockfile(stream);
    return result;
}

int _PHOENIX_fflush_unlocked(FILE* stream) {
    /* TODO */
    stream->error = -1;
    return EOF;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fopen.html */
FILE* fopen(const char* restrict path, const char* restrict mode) {
    /* Find an unused `FILE` object. */
    size_t i;
    while (true) {
        for (i = 0; i < FOPEN_MAX; ++i) {
            if (!files[i].is_open) break;
        }

        if (i == FOPEN_MAX) EFAIL(EMFILE); /* NB: STREAM_MAX is defined to be equal to FOPEN_MAX. */

        /* Try to claim it before another thread does. */
        if (!ftrylockfile(files + i)) break;
    }

    /* Open the file. */
    FILE* result = freopen(path, mode, files + i);

    funlockfile(files + i);
    return result;

fail:
    return NULL;
}

FILE* freopen(const char* restrict path, const char* restrict mode, FILE* restrict stream) {
    flockfile(stream);

    /* TODO: If a signal is caught during this function, then FAIL(EINTR). */

    {
        /* Ignore errors while flushing and closing, except EINTR and (when no path is given) EBADF. */
        int old_errno = errno;
        errno = 0;
        if (_PHOENIX_fflush_unlocked(stream) == EOF && (errno == EINTR || (!path && errno == EBADF))) {
            stream->error = 0;
            EFAIL(errno);
        }
        stream->error = 0;
        stream->eof = 0;
        if (close(stream->fildes) == -1 && (errno == EINTR || (!path && errno == EBADF))) {
            EFAIL(errno);
        }
        errno = old_errno;
    }

    if (!path) path = stream->path;

    int oflag;
    switch (mode[0]) {
        case 'r':
            oflag = O_RDONLY;
            stream->io_mode = IO_READ;
            break;
        case 'w':
            oflag = O_WRONLY | O_CREAT | O_TRUNC;
            stream->io_mode = IO_WRITE;
            break;
        case 'a':
            oflag = O_WRONLY | O_CREAT | O_APPEND;
            stream->io_mode = IO_WRITE;
            break;
        default:
            EFAIL(EINVAL);
    }
    switch (mode[1]) {
        case '\0':
            break;
        case 'b':
            switch (mode[2]) {
                case '\0':
                    break;
                case '+':
                    oflag = (oflag & ~(O_RDONLY | O_WRONLY)) | O_RDWR;
                    stream->io_mode = IO_RW;
                    if (mode[3] == '\0') break;
                    /* Intentional fall-through */
                default:
                    EFAIL(EINVAL);
            }
            break;
        case '+':
            oflag = (oflag & ~(O_RDONLY | O_WRONLY)) | O_RDWR;
            stream->io_mode = IO_RW;
            switch (mode[2]) {
                case '\0':
                    break;
                case 'b':
                    if (mode[3] == '\0') break;
                    /* Intentional fall-through */
                default:
                    EFAIL(EINVAL);
            }
            break;
        default:
            EFAIL(EINVAL);
    }

    if ((stream->fildes = open(path, oflag)) == -1) goto fail;
    stream->is_open = -1;

    stream->path = path;
    stream->char_width = CW_UNSET;
    stream->position.offset = 0;
    /* FIXME: stream->position.mb_parse_state = ...; */
    stream->malloced_buffer = 0;
    stream->buffer = NULL;
    stream->buffer_mode = _IONBF;
    stream->buffer_size = 0;
    stream->buffer_index = 0;
    stream->pushback_index = 0;

    funlockfile(stream);
    return stream;

fail:
    funlockfile(stream);
    return NULL;
}

/* TODO
FILE* fdopen(int fildes, const char* mode);
FILE* fmemopen(void* restrict buf, size_t size, const char* restrict mode);
FILE* open_memstream(char** bufp, size_t* sizep); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/setbuf.html */
void setbuf(FILE* restrict stream, char* restrict buffer) {
    if (buffer) {
        (void)setvbuf(stream, buffer, _IOFBF, BUFSIZ);
    } else {
        (void)setvbuf(stream, NULL, _IONBF, 0);
    }
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/setvbuf.html */
int setvbuf(FILE* restrict stream, char* restrict buffer, int mode, size_t size) {
    bool malloced_buffer = false;

    if (mode == _IONBF) {
        /* If we're not buffering, let's be consistent about it. */
        buffer = NULL;
        size = 0;
    } else if (!buffer && size) {
        /* No buffer given; allocate one instead. */
        malloced_buffer = true;
        buffer = malloc(size);
        if (!buffer) {
            errno = ENOMEM;
            return -1;
        }
    }

    flockfile(stream);

    /* "The setvbuf() function may be used after the stream pointed to by stream is associated with an open file
       but before any other operation (other than an unsuccessful call to setvbuf()) is performed on the stream."
       Therefore, we can assume that the buffer is currently empty, and we don't need to flush it. */

    if (stream->malloced_buffer) {
        free(stream->buffer);
    }

    stream->malloced_buffer = malloced_buffer;
    stream->buffer = buffer;
    stream->buffer_mode = mode;
    stream->buffer_size = size;
    stream->buffer_index = 0;

    funlockfile(stream);

    return 0;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fileno.html */
int fileno(FILE* stream) {
    flockfile(stream); /* Required by https://pubs.opengroup.org/onlinepubs/9699919799/functions/flockfile.html */
    int fildes = stream->fildes;
    funlockfile(stream);
    if (fildes < 0) EFAIL(EBADF);
    return fildes;

fail:
    return -1;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/flockfile.html */
void flockfile(FILE* stream) {
    /* FIXME: POSIX requires this to be a re-entrant lock. */
    while (atomic_flag_test_and_set_explicit(&stream->lock, memory_order_acq_rel)) {
        sleep(0);
    }
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ftrylockfile.html */
int ftrylockfile(FILE* stream) {
    return atomic_flag_test_and_set_explicit(&stream->lock, memory_order_acq_rel);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/funlockfile.html */
void funlockfile(FILE* stream) {
    atomic_flag_clear_explicit(&stream->lock, memory_order_release);
}


/* Formatted input/output */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/dprintf.html */
int dprintf(int fildes, const char* restrict format, ...) {
    va_list args;
    va_start(args, format);
    int result = vdprintf(fildes, format, args);
    va_end(args);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fprintf.html */
int fprintf(FILE* restrict stream, const char* restrict format, ...) {
    va_list args;
    va_start(args, format);
    int result = vfprintf(stream, format, args);
    va_end(args);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fscanf.html */
int fscanf(FILE* restrict stream, const char* restrict format, ...) {
    va_list args;
    va_start(args, format);
    int result = vfscanf(stream, format, args);
    va_end(args);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/printf.html */
int printf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    int result = vprintf(format, args);
    va_end(args);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/scanf.html */
int scanf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    int result = vscanf(format, args);
    va_end(args);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/snprintf.html */
int snprintf(char* restrict s, size_t n, const char* restrict format, ...) {
    va_list args;
    va_start(args, format);
    int result = vsnprintf(s, n, format, args);
    va_end(args);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/sprintf.html */
int sprintf(char* restrict s, const char* restrict format, ...) {
    va_list args;
    va_start(args, format);
    int result = vsprintf(s, format, args);
    va_end(args);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/sscanf.html */
int sscanf(const char* restrict s, const char* restrict format, ...) {
    va_list args;
    va_start(args, format);
    int result = vsscanf(s, format, args);
    va_end(args);
    return result;
}

/* A common implementation for all the printf variants.
 * put_char should be a statement that prints the character `c` in the appropriate way.
 * finalize should be a statement that does any necessary cleanup before the function returns with no error. */
#define PRINTF_BODY_GENERIC(put_char, finalize) \
    struct lconv* locale_conv = localeconv(); \
    char c; \
    int bytes_written = 0; \
    const char* old_format; \
\
    va_list positioned_args[NL_ARGMAX]; \
    long positioned_args_count = find_positioned_args(format, args, positioned_args); \
    if (positioned_args_count < 0) EFAIL(EINVAL); \
\
    while ((c = *format++)) { \
        FormatSpec spec = {0}; \
        if (c == '%') { \
            old_format = format; \
            if (parse_format_spec(&format, &spec)) { \
                /* POSIX says this branch is UB. Print the conversion specifier to hopefully make the error obvious. */ \
                goto put; \
            } \
\
            switch (spec.flags & FSF_TEXT_TYPE) { \
            case FSF_TEXT_INTEGER: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_FLOAT_LOWER: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_FLOAT_UPPER: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_FLOAT_SCI_LOWER: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_FLOAT_SCI_UPPER: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_FLOAT_FLEX_LOWER: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_FLOAT_FLEX_UPPER: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_CHAR: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_STRING: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_POINTER: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_SCANSET: \
                /* This format spec is invalid in a printf() format string. Print it verbatim to hopefully make the error obvious. */ \
                format = old_format; \
                goto put; \
                break; \
\
            case FSF_TEXT_COUNT: \
                /* TODO */ \
                break; \
\
            case FSF_TEXT_PERCENT: \
                goto put; \
                break; \
\
            default: \
                /* Unrecognized format spec type, even though it was parsed successfully. This is a bug in libc. */ \
                EFAIL(EINTERNAL); \
            } \
        } else { \
put: \
            put_char; \
            if (bytes_written++ == INT_MAX) EFAIL(EOVERFLOW); \
        } \
    } \
\
    finalize; \
\
    while (positioned_args_count--) { \
        va_end(positioned_args[positioned_args_count]); \
    } \
\
    return bytes_written; \
\
fail: \
    return -1;

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/vdprintf.html */
int vdprintf(int fildes, const char* restrict format, va_list args) {
    PRINTF_BODY_GENERIC(if (write(fildes, &c, 1) == -1) goto fail,)
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/vfprintf.html */
int vfprintf(FILE* restrict stream, const char* restrict format, va_list args) {
    PRINTF_BODY_GENERIC(if (fputc(c, stream) == EOF) goto fail,)
}

/* TODO
int vfscanf(FILE* restrict stream, const char* restrict format, va_list args);
*/

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/vprintf.html */
int vprintf(const char* format, va_list args) {
    return vfprintf(stdout, format, args);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/vscanf.html */
int vscanf(const char* format, va_list args) {
    return vfscanf(stdin, format, args);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/vsnprintf.html */
int vsnprintf(char* restrict s, size_t n, const char* restrict format, va_list args) {
    PRINTF_BODY_GENERIC(if (n > 1) { *s++ = c; --n; }, if (n) { *s = '\0'; })
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/vsprintf.html */
int vsprintf(char* restrict s, const char* restrict format, va_list args) {
    PRINTF_BODY_GENERIC(*s++ = c, *s = '\0')
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/vsscanf.html */
int vsscanf(const char* restrict s, const char* restrict format, va_list args) {
    int sign;
    int radix, dec_digits, hex_digits;
    uintmax_t i_acc;
    va_list args_temp;
    void* out = NULL;

    /* FIXME: Allow the "optional assignment-allocation character 'm'", as described by POSIX. */

    int args_filled = 0;
    const char* const s_start = s;
    char c = *s++;
    while (c && *format) {
        char fc = *format++;

        /* A single whitespace character in the format string matches 0 or more whitespace characters
           in the input string. */
        if (isspace(fc)) {
            while (isspace(c)) {
                c = *s++;
            }
            continue;
        }

        /* A non-whitespace, non-format-specifier character must match exactly. */
        if (fc != '%') {
            if (c != fc) {
                goto matching_break;
            }
            c = *s++;
            continue;
        }

        /* Format specifiers */
        bool store_arg = true;
        if (*format == '*') {
            store_arg = false;
            ++format;
        }
        FormatSpec spec = {0};
        if (parse_format_spec(&format, &spec)) EFAIL(EINVAL);

        size_t width_counter = SIZE_MAX;
        if (spec.flags & FSF_HAS_WIDTH) {
            if (spec.flags & FSF_WIDTH_FROM_ARG) EFAIL(EINVAL);
            width_counter = spec.width;
        }

        switch (spec.flags & FSF_TEXT_TYPE) {
        case FSF_TEXT_INTEGER:
        case FSF_TEXT_POINTER:
            while (isspace(c)) {
                c = *s++;
            }

            i_acc = 0;
            sign = 1;
            if (width_counter != 0) {
                switch (c) {
                case '-':
                    sign = -1;
                    /* Intentional fall-through */
                case '+':
                    c = *s++;
                    --width_counter;
                    break;
                }
            }
            switch (spec.flags & FSF_RADIX) {
            case FSF_ANY_RADIX:
                radix = 10;
                if (width_counter-- && c == '0') {
                    radix = 8;
                    c = *s++;
                    if (width_counter-- && (c == 'x' || c == 'X')) {
                        radix = 16;
                        c = *s++;
                    }
                }
                break;
            case FSF_DECIMAL:
                radix = 10;
                break;
            case FSF_OCTAL:
                radix = 8;
                break;
            case FSF_HEX_LOWER:
            case FSF_HEX_UPPER:
                radix = 16;
                if (width_counter >= 2 && c == '0' && (*s == 'x' || *s == 'X')) {
                    /* A hexadecimal number can optionally start with "0x". */
                    s++;
                    c = *s++;
                    width_counter -= 2;
                }
                break;
            default: /* Should be unreachable */
                EFAIL(EINTERNAL);
            }
            if (radix <= 10) {
                dec_digits = radix;
                hex_digits = 0;
            } else {
                dec_digits = 10;
                hex_digits = radix - 10;
            }
            while (width_counter--) {
                if (c >= '0' && c < '0' + dec_digits) {
                    i_acc = i_acc * radix + (c - '0');
                    c = *s++;
                } else if (c >= 'a' && c < 'a' + hex_digits) {
                    i_acc = i_acc * radix + (c - 'a' + 10);
                    c = *s++;
                } else if (c >= 'A' && c < 'A' + hex_digits) {
                    i_acc = i_acc * radix + (c - 'A' + 10);
                    c = *s++;
                }
            }
            if (store_arg) {
                if (spec.argpos) {
                    va_copy(args_temp, args);
                    for (long i = 0; i < spec.argpos; ++i) {
                        out = va_arg(args_temp, void*);
                    }
                    va_end(args_temp);
                } else {
                    out = va_arg(args, void*);
                }

                switch (spec.flags & FSF_ARG_TYPE) {
                case FSF_ARG_DEFAULT:
                    if ((spec.flags & FSF_TEXT_TYPE) == FSF_TEXT_POINTER) {
                        *(void**)out = (void*)(sign * (intmax_t)i_acc);
                    } else if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *(int*)out = sign * (intmax_t)i_acc;
                    } else {
                        *(unsigned int*)out = sign * i_acc;
                    }
                    break;
                case FSF_ARG_CHAR:
                    if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *(signed char*)out = sign * (intmax_t)i_acc;
                    } else {
                        *(unsigned char*)out = sign * i_acc;
                    }
                    break;
                case FSF_ARG_SHORT:
                    if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *(short*)out = sign * (intmax_t)i_acc;
                    } else {
                        *(unsigned short*)out = sign * i_acc;
                    }
                    break;
                case FSF_ARG_LONG:
                    if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *(long*)out = sign * (intmax_t)i_acc;
                    } else {
                        *(unsigned long*)out = sign * i_acc;
                    }
                    break;
                case FSF_ARG_LONG_LONG:
                    if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *(long long*)out = sign * (intmax_t)i_acc;
                    } else {
                        *(unsigned long*)out = sign * i_acc;
                    }
                    break;
                case FSF_ARG_INTMAX_T:
                    if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *(intmax_t*)out = sign * (intmax_t)i_acc;
                    } else {
                        *(uintmax_t*)out = sign * i_acc;
                    }
                    break;
                case FSF_ARG_SIZE_T:
                    *(size_t*)out = sign * i_acc;
                    break;
                case FSF_ARG_PTRDIFF_T:
                    *(ptrdiff_t*)out = sign * (intmax_t)i_acc;
                    break;

                default:
                    goto matching_break;
                }
                ++args_filled;
            }
            break;
        case FSF_TEXT_FLOAT_LOWER:
        case FSF_TEXT_FLOAT_UPPER:
        case FSF_TEXT_FLOAT_SCI_LOWER:
        case FSF_TEXT_FLOAT_SCI_UPPER:
        case FSF_TEXT_FLOAT_FLEX_LOWER:
        case FSF_TEXT_FLOAT_FLEX_UPPER:
            while (isspace(c)) {
                c = *s++;
            }

            /* FIXME
            A valid floating point number for strtod using the "C" locale is formed by an optional
            sign character (+ or -), followed by one of:
            - A sequence of digits, optionally containing a decimal-point character (.), optionally
               followed by an exponent part (an e or E character followed by an optional sign and a
               sequence of digits).
            - A 0x or 0X prefix, then a sequence of hexadecimal digits (as in isxdigit) optionally
               containing a period which separates the whole and fractional number parts.
               Optionally followed by a power of 2 exponent (a p or P character followed by an
               optional sign and a sequence of hexadecimal digits).
            - INF or INFINITY (ignoring case).
            - NAN or NANsequence (ignoring case), where sequence is a sequence of characters, where
               each character is either an alphanumeric character (as in isalnum) or the underscore
               character (_).
            */
            ++args_filled; /* TODO: Should be done only if the argument is actually used */
            break;
        case FSF_TEXT_CHAR:
            if (!(spec.flags & FSF_HAS_WIDTH)) {
                width_counter = 1;
            }
            if (store_arg) {
                if (spec.argpos) {
                    va_copy(args_temp, args);
                    for (long i = 0; i < spec.argpos; ++i) {
                        out = va_arg(args_temp, void*);
                    }
                    va_end(args_temp);
                } else {
                    out = va_arg(args, void*);
                }
            }
            while (width_counter--) {
                if (store_arg) {
                    switch (spec.flags & FSF_ARG_TYPE) {
                    case FSF_ARG_DEFAULT:
                        *(char*)out = c;
                        out = (char*)out + 1;
                        break;
                    case FSF_ARG_LONG:
                        /* FIXME: "The conversion specifiers lc, ls, and l[ perform multibyte-to-wide
                        character conversion as if by calling mbrtowc() with an mbstate_t object
                        initialized to zero before the first character is converted." */
                        *(wchar_t*)out = c;
                        out = (wchar_t*)out + 1;
                        break;
                    default:
                        goto matching_break;
                    }
                }
                c = *s++;
            }
            if (store_arg) {
                ++args_filled;
            }
            break;
        case FSF_TEXT_STRING:
            while (isspace(c)) {
                c = *s++;
            }

            if (store_arg) {
                if (spec.argpos) {
                    va_copy(args_temp, args);
                    for (long i = 0; i < spec.argpos; ++i) {
                        out = va_arg(args_temp, void*);
                    }
                    va_end(args_temp);
                } else {
                    out = va_arg(args, void*);
                }
            }
            while (width_counter-- && c && !isspace(c)) {
                if (store_arg) {
                    switch (spec.flags & FSF_ARG_TYPE) {
                    case FSF_ARG_DEFAULT:
                        *(char*)out = c;
                        out = (char*)out + 1;
                        break;
                    case FSF_ARG_LONG:
                        /* FIXME: "The conversion specifiers lc, ls, and l[ perform multibyte-to-wide
                        character conversion as if by calling mbrtowc() with an mbstate_t object
                        initialized to zero before the first character is converted." */
                        *(wchar_t*)out = c;
                        out = (wchar_t*)out + 1;
                        break;
                    default:
                        goto matching_break;
                    }
                }
                c = *s++;
            }
            if (store_arg) {
                *(char*)out = '\0';
                ++args_filled;
            }
            break;
        case FSF_TEXT_SCANSET:
            if (store_arg) {
                if (spec.argpos) {
                    va_copy(args_temp, args);
                    for (long i = 0; i < spec.argpos; ++i) {
                        out = va_arg(args_temp, void*);
                    }
                    va_end(args_temp);
                } else {
                    out = va_arg(args, void*);
                }
            }
            while (width_counter-- && c) {
                const char* scanner = spec.scanner;
                do {
                    if (c == *scanner++) {
                        if (spec.flags & FSF_SCANSET_NEGATED) {
                            goto scanset_break;
                        } else {
                            goto scanset_store;
                        }
                    }
                } while (*scanner != ']');
scanset_store:
                if (store_arg) {
                    switch (spec.flags & FSF_ARG_TYPE) {
                    case FSF_ARG_DEFAULT:
                        *(char*)out = c;
                        out = (char*)out + 1;
                        break;
                    case FSF_ARG_LONG:
                        /* FIXME: "The conversion specifiers lc, ls, and l[ perform multibyte-to-wide
                        character conversion as if by calling mbrtowc() with an mbstate_t object
                        initialized to zero before the first character is converted." */
                        *(wchar_t*)out = c;
                        out = (wchar_t*)out + 1;
                        break;
                    default:
                        goto matching_break;
                    }
                }
                c = *s++;
            }
scanset_break:
            if (store_arg) {
                *(char*)out = '\0';
                ++args_filled;
            }
            break;
        case FSF_TEXT_COUNT:
            if (store_arg) {
                if (spec.argpos) {
                    va_copy(args_temp, args);
                    for (long i = 0; i < spec.argpos; ++i) {
                        out = va_arg(args_temp, void*);
                    }
                    va_end(args_temp);
                } else {
                    out = va_arg(args, void*);
                }

                switch (spec.flags & FSF_ARG_TYPE) {
                case FSF_ARG_DEFAULT:
                    *(int*)out = s - s_start;
                    break;
                case FSF_ARG_CHAR:
                    *(signed char*)out = s - s_start;
                    break;
                case FSF_ARG_SHORT:
                    *(short*)out = s - s_start;
                    break;
                case FSF_ARG_LONG:
                    *(long*)out = s - s_start;
                    break;
                case FSF_ARG_LONG_LONG:
                    *(long long*)out = s - s_start;
                    break;
                case FSF_ARG_INTMAX_T:
                    *(intmax_t*)out = s - s_start;
                    break;
                case FSF_ARG_SIZE_T:
                    *(size_t*)out = s - s_start;
                    break;
                case FSF_ARG_PTRDIFF_T:
                    *(ptrdiff_t*)out = s - s_start;
                    break;
                default:
                    goto matching_break;
                }
                ++args_filled;
            }
            break;
        case FSF_TEXT_PERCENT:
            while (isspace(c)) {
                c = *s++;
            }

            if (!(width_counter-- && c == '%')) {
                c = *s++;
                goto matching_break;
            }
            break;
        }
    }
matching_break:
    return args_filled; /* TODO: If EOF is reached before any data can be read, return EOF. */

fail:
    return EOF;
}


/* Character input/output */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fgetc.html */
int fgetc(FILE* stream) {
    unsigned char buf[1];
    if (fread(&buf, 1, 1, stream) < 1) {
        return EOF;
    }
    return buf[0];
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/getc_unlocked.html */
int getc_unlocked(FILE* stream) {
    unsigned char buf[1];
    if (_PHOENIX_fread_unlocked(&buf, 1, 1, stream) < 1) {
        return EOF;
    }
    return buf[0];
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/getc.html */
int getc(FILE* stream) {
    return fgetc(stream);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/getchar.html */
int getchar(void) {
    return fgetc(stdin);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/getchar_unlocked.html */
int getchar_unlocked(void) {
    return getc_unlocked(stdin);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fgets.html */
char* fgets(char* restrict str, int num, FILE* restrict stream) {
    if (num == 0) EFAIL(EINVAL); /* No room in the buffer for even the null terminator */

    flockfile(stream);

    /* "If the end-of-file condition is encountered before any bytes are read, the contents of the array pointed to by s shall not be changed." */
    if (stream->eof) {
        str = NULL;
        goto end;
    }

    int i = 0;
    while (i < num - 1) {
        if (stream->eof) break;
        int c = getc_unlocked(stream);
        if (c == EOF) {
            if (stream->error || i == 0) {
                str = NULL;
                goto end;
            }
            break;
        }
        str[i++] = UCHAR_TO_CHAR(c);
        if (c == '\n') break;
    }
    str[i] = '\0';

end:
    funlockfile(stream);
    return str;

fail:
    return NULL;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/gets.html */
char* gets(char* str) {
    flockfile(stdin);

    if (stdin->eof) {
        str = NULL;
        goto end;
    }

    int i = 0;
    for (;;) {
        if (stdin->eof) break;
        int c = getchar_unlocked();
        if (c == EOF) {
            if (stdin->error || i == 0) {
                str = NULL;
                goto end;
            }
            break;
        }
        if (c == '\n') break;
        str[i++] = UCHAR_TO_CHAR(c);
    }
    str[i] = '\0';

end:
    funlockfile(stdin);
    return str;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/getdelim.html */
ssize_t getdelim(char** restrict lineptr, size_t* restrict size, int delimiter, FILE* restrict stream) {
    int allocated = 0;

    flockfile(stream);

    if (stream->char_width == CW_WIDE) EFAIL(EINVAL);
    stream->char_width = CW_NARROW;

    if (!lineptr || !size) EFAIL(EINVAL);
    if (stream->eof) goto eof;

    if (!*size) {
        *size = 16;
        if (*lineptr) {
            *lineptr = realloc(*lineptr, *size);
            allocated = 1;
        }
    }
    if (!*lineptr) {
        *lineptr = malloc(*size);
        allocated = 1;
    }

    size_t bytes_read = 0;
    int c;
    while ((c = getc_unlocked(stream)) != EOF) {
        if (++bytes_read == *size) {
            *size *= 2;
            *lineptr = realloc(*lineptr, *size);
        }
        (*lineptr)[bytes_read - 1] = UCHAR_TO_CHAR(c);
        if (c == delimiter) break;
    }

    if (c == EOF && (stream->error || (stream->eof && bytes_read == 0))) goto eof;

    (*lineptr)[bytes_read] = '\0';

    funlockfile(stream);
    return bytes_read;

fail:
    stream->error = -1;
eof:
    funlockfile(stream);
    if (allocated) free(*lineptr);
    return -1;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/getline.html */
ssize_t getline(char** restrict lineptr, size_t* restrict size, FILE* restrict stream) {
    return getdelim(lineptr, size, '\n', stream);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fputc.html */
int fputc(int ch, FILE* stream) {
    if (fwrite(&ch, 1, 1, stream) < 1) return EOF;
    return ch;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/putc.html */
int putc(int ch, FILE* stream) {
    return fputc(ch, stream);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/putc_unlocked.html */
int putc_unlocked(int ch, FILE* stream) {
    if (_PHOENIX_fwrite_unlocked(&ch, 1, 1, stream) < 1) return EOF;
    return ch;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/putchar.html */
int putchar(int ch) {
    return fputc(ch, stdout);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/putchar_unlocked.html */
int putchar_unlocked(int ch) {
    return putc_unlocked(ch, stdout);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fputs.html */
int fputs(const char* str, FILE* stream) {
    int result = 0;

    flockfile(stream);

    if (stream->char_width == CW_WIDE) {
        stream->error = -1;
        EFAIL(EINVAL);
    }
    stream->char_width = CW_NARROW;

    if (!stream->buffer_size || stream->buffer_mode == _IONBF) {
        /* The stream is unbuffered, so just send the string. */
        /* FIXME: Implement this, ideally by sending the whole string at once. */
        stream->error = -1;
        result = EOF;
        goto done;
    }

    while (*str) {
        /* Buffer as many characters as possible. */
        while (stream->buffer_index < stream->buffer_size) {
            stream->buffer[stream->buffer_index++] = *str++;
            if (!*str) goto done;
        }

        /* Flush the buffer before continuing. */
        if (_PHOENIX_fflush_unlocked(stream)) {
            result = EOF;
            goto done;
        }
    }

done:
fail:
    funlockfile(stream);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/puts.html */
int puts(const char* str) {
    flockfile(stdout);
    int result;
    if (fputs(str, stdout) < 0) result = EOF;
    else result = putchar_unlocked('\n');
    funlockfile(stdout);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ungetc.html */
int ungetc(int ch, FILE* stream) {
    if (ch == EOF) return EOF; /* Ungetting nothing */

    flockfile(stream);

    int result = EOF;

    if (stream->char_width == CW_WIDE) {
        stream->error = -1;
        EFAIL(EINVAL);
    }
    stream->char_width = CW_NARROW;

    if (stream->pushback_index == sizeof(stream->pushback_buffer.c)) goto fail; /* Buffer already full */

    stream->pushback_buffer.c[stream->pushback_index++] = ch;
    stream->eof = false;
    result = ch;

fail:
    funlockfile(stream);
    return result;
}

/* Direct input/output */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fread.html */
size_t fread(void* restrict buffer, size_t size, size_t count, FILE* restrict stream) {
    if (size == 0 || count == 0) return 0;

    flockfile(stream);
    size_t result = _PHOENIX_fread_unlocked(buffer, size, count, stream);
    funlockfile(stream);
    return result;
}

size_t _PHOENIX_fread_unlocked(void* restrict buffer, size_t size, size_t count, FILE* restrict stream) {
    size_t bytes_read = 0;

    if (stream->char_width == CW_WIDE) {
        stream->error = -1;
        EFAIL(EINVAL);
    }
    stream->char_width = CW_NARROW;

    if (size == 0 || count == 0) return 0;
    if (!stream->is_open || !(stream->io_mode & IO_READ)) {
        stream->error = -1;
        EFAIL(EBADF);
    }
    if (stream->eof) goto fail;

    size_t total_size = size * count;

    size_t pushback_bytes = stream->pushback_index < total_size ? stream->pushback_index : total_size;
    memcpy(buffer, stream->pushback_buffer.c, pushback_bytes);
    bytes_read += pushback_bytes;
    stream->pushback_index -= pushback_bytes;

    bytes_read += read(stream->fildes, buffer, total_size - bytes_read);

fail:
    return bytes_read / size;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fwrite.html */
size_t fwrite(const void* restrict buffer, size_t size, size_t count, FILE* restrict stream) {
    flockfile(stream);
    size_t result = _PHOENIX_fwrite_unlocked(buffer, size, count, stream);
    funlockfile(stream);
    return result;
}

size_t _PHOENIX_fwrite_unlocked(const void* restrict buffer, size_t size, size_t count, FILE* restrict stream) {
    size_t bytes_remaining = size * count;

    if (stream->char_width == CW_WIDE) {
        stream->error = -1;
        EFAIL(EINVAL);
    }

    if (size == 0 || count == 0) return 0;

    size_t obj_buffer_index = 0;
    while (bytes_remaining) {
        if (!stream->buffer_size || stream->buffer_mode == _IONBF) {
            /* The stream is unbuffered, so just send one character at a time. */
            /* TODO */
            stream->error = -1;
            break;
        }

        /* Flush the buffer if it's full. */
        if (stream->buffer_index >= stream->buffer_size && _PHOENIX_fflush_unlocked(stream)) {
            /* The stream's error flag is already set. */
            break;
        }

        /* Buffer as many bytes as possible. */
        size_t obj_buffer_size_remaining = size - obj_buffer_index;
        size_t file_buffer_size_remaining = stream->buffer_size - stream->buffer_index;
        size_t bytes_to_write = obj_buffer_size_remaining < file_buffer_size_remaining
            ? obj_buffer_size_remaining
            : file_buffer_size_remaining;
        memcpy(
            stream->buffer + stream->buffer_index,
            (const unsigned char*)buffer + obj_buffer_index,
            bytes_to_write
        );
        stream->buffer_index += bytes_to_write;
        obj_buffer_index += bytes_to_write;
        if (obj_buffer_index >= size) obj_buffer_index -= size;
        bytes_remaining -= bytes_to_write;
    }

fail:
    return (size * count - bytes_remaining) / size;
}


/* File positioning */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fgetpos.html */
int fgetpos(FILE* restrict stream, fpos_t* restrict pos) {
    flockfile(stream);

    /* FIXME: If "The file descriptor underlying stream is not valid." */
    if (false) EFAIL(EBADF);

    /* FIXME: If "The file descriptor underlying stream is associated with a pipe, FIFO, or socket." */
    if (false) EFAIL(ESPIPE);

    *pos = stream->position;

    /* Correct the position for buffered writes and calls to unget. */
    pos->offset += stream->buffer_index;
    pos->offset -= stream->pushback_index;

    funlockfile(stream);
    return 0;

fail:
    funlockfile(stream);
    return -1;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fsetpos.html */
int fsetpos(FILE* stream, const fpos_t* pos) {
    flockfile(stream);

    if (_PHOENIX_fflush_unlocked(stream)) goto fail;

    stream->position = *pos;
    stream->pushback_buffer.wc = WEOF;
    stream->pushback_index = 0;
    stream->eof = false;

    funlockfile(stream);
    return 0;

fail:
    funlockfile(stream);
    return -1;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fseek.html */
int fseek(FILE* stream, long offset, int whence) {
    flockfile(stream);
    int result = _PHOENIX_fseek_unlocked(stream, offset, whence);
    funlockfile(stream);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fseeko.html */
int fseeko(FILE* stream, off_t offset, int whence) {
    flockfile(stream);
    int result = _PHOENIX_fseeko_unlocked(stream, offset, whence);
    funlockfile(stream);
    return result;
}

#define FSEEK_GENERIC(fn_name, offset_t, offset_t_max, offset_t_min) \
int fn_name(FILE* stream, offset_t offset, int whence) { \
    /* FIXME: If "The file descriptor underlying `stream` is associated with a pipe, FIFO, or socket." */ \
    if (false) EFAIL(ESPIPE); \
\
    if (_PHOENIX_fflush_unlocked(stream)) goto fail; \
\
    switch (whence) { \
    case SEEK_SET: \
        stream->position.offset = offset; \
        break; \
    case SEEK_CUR: \
        if ( \
            (stream->position.offset <= offset_t_max && offset > offset_t_max - (long)stream->position.offset) || \
            stream->position.offset + offset > offset_t_max \
        ) { \
            /* Seeking beyond the range of an `offset_t` */ \
            EFAIL(EOVERFLOW); \
        } \
        stream->position.offset += offset; \
        break; \
    case SEEK_END: \
        if ( \
            (stream->length <= LONG_MAX && offset > offset_t_max - (long)stream->length) || \
            stream->length + offset > offset_t_max \
        ) { \
            /* Seeking beyond the range of a `long` */ \
            EFAIL(EOVERFLOW); \
        } \
        stream->position.offset = stream->length + offset; \
        break; \
    default: \
        /* Unrecognized seek origin */ \
        EFAIL(EINVAL); \
    } \
\
    stream->pushback_buffer.wc = WEOF; \
    stream->pushback_index = 0; \
    stream->eof = false; \
\
    return 0; \
\
fail: \
    return -1; \
}

FSEEK_GENERIC(_PHOENIX_fseek_unlocked, long, LONG_MAX, LONG_MIN)
FSEEK_GENERIC(_PHOENIX_fseeko_unlocked, off_t, OFF_MAX, OFF_MIN)

#define FTELL_GENERIC(fn_name, offset_t, offset_t_max) \
offset_t fn_name(FILE* stream) { \
    flockfile(stream); \
\
    /* FIXME: If "The file descriptor underlying `stream` is not an open file descriptor." */ \
    if (false) EFAIL(EBADF); \
\
    /* FIXME: If "The file descriptor underlying `stream` is associated with a pipe, FIFO, or socket." */ \
    if (false) EFAIL(ESPIPE); \
\
    /* Calculate current offset, including buffered writes and calls to unget. */ \
    offset_t offset = stream->position.offset + stream->buffer_index - stream->pushback_index; \
\
    if (offset > offset_t_max) EFAIL(EOVERFLOW); \
\
    funlockfile(stream); \
    return offset; \
\
fail: \
    funlockfile(stream); \
    return -1L; \
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ftell.html */
FTELL_GENERIC(ftell, long, LONG_MAX)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ftello.html */
FTELL_GENERIC(ftello, off_t, OFF_MAX)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/rewind.html */
void rewind(FILE* stream) {
    flockfile(stream);

    (void)_PHOENIX_fseek_unlocked(stream, 0L, SEEK_SET);
    stream->error = false;

    funlockfile(stream);
}


/* Error-handling */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/clearerr.html */
void clearerr(FILE* stream) {
    flockfile(stream);
    stream->eof = false;
    stream->error = false;
    funlockfile(stream);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/feof.html */
int feof(FILE* stream) {
    flockfile(stream);
    int result = stream->eof;
    funlockfile(stream);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ferror.html */
int ferror(FILE* stream) {
    flockfile(stream);
    int result = stream->error;
    funlockfile(stream);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/perror.html */
void perror(const char* s) {
    flockfile(stderr);
    CharWidth width = stderr->char_width;
    switch (width) {
    case CW_UNSET:
    case CW_NARROW:
        if (s) {
            fprintf(stderr, "%s: ", s);
        }
        fprintf(stderr, "%s\n", strerror(errno));
        break;
    case CW_WIDE:
        if (s) {
            fwprintf(stderr, L"%s: ", s);
        }
        fwprintf(stderr, L"%s\n", strerror(errno));
        break;
    }
    stderr->char_width = width;
    funlockfile(stderr);
}

/* Terminals */
/* TODO
char* ctermid(char* s); */

/* Processes */
/* TODO
FILE* popen(const char* command, const char* mode);
int pclose(FILE* stream); */


/*
 * Parses a format specification, as found in calls to printf and scanf.
 *
 * Parameters:
 * `format`: A pointer to the format string pointer. The format string pointer should be pointing
 *     just past the '%' that begins the format specification (and, in scanf and friends, if the
 *     format specification begins with '*', after that asterisk as well). When this function
 *     returns, the pointer will point at the character immediately following the format
 *     specification.
 * `spec`: A FormatSpec object to be initialized according to the format specification.
 *
 * Returns:
 * 0 on success, nonzero if the format string is malformed. If a nonzero value is returned,
 * `format` is unchanged.
 */
static int parse_format_spec(const char* restrict* format, FormatSpec* restrict spec) {
    const char* restrict* orig_format = format;

    spec->argpos = 0;
    spec->flags = 0;

    /* Argument position */
    if (isdigit(**format) && **format != '0') {
        spec->argpos = strtol(*format, (char**)format, 10);
        if (spec->argpos > NL_ARGMAX) goto fail;
        if (*(*format)++ != '$') goto fail;
    }

    /* Flags */
    for (; **format; ++*format) {
        switch (**format) {
        case '\'':
            spec->flags |= FSF_THOUSANDS;
            break;
        case '-':
            spec->flags |= FSF_JUSTIFY_LEFT;
            break;
        case '+':
            spec->flags |= FSF_FORCE_SIGN;
            break;
        case ' ':
            spec->flags |= FSF_SPACE_AS_SIGN;
            break;
        case '#':
            spec->flags |= FSF_DECORATE;
            break;
        case '0':
            spec->flags |= FSF_PAD_WITH_ZERO;
            break;

        default:
            goto flags_break;
        }
    }
flags_break:

    /* Width */
    if (**format == '*') {
        ++*format;
        spec->flags |= FSF_HAS_WIDTH | FSF_WIDTH_FROM_ARG;
    } else if (isdigit(**format)) {
        spec->flags |= FSF_HAS_WIDTH;
        unsigned long width = strtoul(*format, (char**)format, 10); /* Always positive because we handled the '-' flag above */
        spec->width = width > SIZE_MAX ? SIZE_MAX : (size_t)width;
    }

    /* Precision */
    if (**format == '.') {
        spec->flags |= FSF_HAS_PRECISION;
        if (*(++*format) == '*') {
            spec->flags |= FSF_PRECISION_FROM_ARG;

            if (isdigit(**format)) {
                spec->precision_argpos = strtol(*format, (char**)format, 10);
                if (spec->precision_argpos > NL_ARGMAX) goto fail;
                if (*(*format)++ != '$') goto fail;
            } else {
                spec->precision_argpos = 0;
            }
        } else {
            long precision = strtol(*format, (char**)format, 10);
            if (precision < 0) spec->flags &= ~FSF_HAS_PRECISION;
            else spec->precision = (unsigned long)precision > SIZE_MAX ? SIZE_MAX : (size_t)precision;
        }
    }

    /* Length */
    switch (**format) {
    case 'h':
        if (*(++*format) == 'h') {
            /* %...hh... */
            ++*format;
            spec->flags |= FSF_ARG_CHAR;
        } else {
            spec->flags |= FSF_ARG_SHORT;
        }
        ++*format;
        break;
    case 'l':
        if (*(++*format) == 'l') {
            /* %...ll... */
            ++*format;
            spec->flags |= FSF_ARG_LONG_LONG;
        } else {
            spec->flags |= FSF_ARG_LONG;
        }
        ++*format;
        break;
    case 'j':
        spec->flags |= FSF_ARG_INTMAX_T;
        ++*format;
        break;
    case 'z':
        spec->flags |= FSF_ARG_SIZE_T;
        ++*format;
        break;
    case 't':
        spec->flags |= FSF_ARG_PTRDIFF_T;
        ++*format;
        break;
    case 'L':
        spec->flags |= FSF_ARG_LONG_DOUBLE;
        ++*format;
        break;
    }

    /* Specifier */
    switch (*(*format)++) {
    case 'd':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        return 0;
    case 'i':
        spec->flags |= FSF_SIGNED | FSF_ANY_RADIX | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        return 0;
    case 'o':
        spec->flags |= FSF_UNSIGNED | FSF_OCTAL | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        return 0;
    case 'u':
        spec->flags |= FSF_UNSIGNED | FSF_DECIMAL | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        return 0;
    case 'x':
        spec->flags |= FSF_UNSIGNED | FSF_HEX_LOWER | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        return 0;
    case 'X':
        spec->flags |= FSF_UNSIGNED | FSF_HEX_UPPER | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        return 0;
    case 'f':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT_LOWER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 6;
        return 0;
    case 'F':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT_UPPER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 6;
        return 0;
    case 'e':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT_SCI_LOWER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 6;
        return 0;
    case 'E':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT_SCI_UPPER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 6;
        return 0;
    case 'g':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT_FLEX_LOWER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 6;
        return 0;
    case 'G':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT_FLEX_UPPER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 6;
        return 0;
    case 'a':
        spec->flags |= FSF_SIGNED | FSF_HEX_LOWER | FSF_TEXT_FLOAT_SCI_LOWER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 6;
        return 0;
    case 'A':
        spec->flags |= FSF_SIGNED | FSF_HEX_UPPER | FSF_TEXT_FLOAT_SCI_UPPER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 6;
        return 0;
    case 'c':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_CHAR;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        return 0;
    case 'C':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_CHAR;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        if (!(spec->flags & FSF_ARG_TYPE)) spec->flags |= FSF_ARG_LONG;
        return 0;
    case 's':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_STRING;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = SIZE_MAX;
        return 0;
    case 'S':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_STRING;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = SIZE_MAX;
        if (!(spec->flags & FSF_ARG_TYPE)) spec->flags |= FSF_ARG_LONG;
        return 0;
    case '[':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_SCANSET;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        return parse_scanset(format, spec);
    case 'p':
        spec->flags |= FSF_UNSIGNED | FSF_HEX_LOWER | FSF_TEXT_POINTER;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        return 0;
    case 'n':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_COUNT;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 0;
        return 0;
    case '%':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_PERCENT;
        if (!(spec->flags & FSF_HAS_PRECISION)) spec->precision = 1;
        return 0;
    }

fail:
    /* Invalid format specification */
    format = orig_format;
    return -1;
}

/*
 * Parses a scanset, as found in calls to scanf.
 *
 * Parameters:
 * `format`: A pointer to the format string pointer. The format string pointer should be pointing
 *     just past the '[' that begins the scanset. When this function returns, the pointer will point
 *     at the character immediately following the scanset.
 * `spec`: A FormatSpec object to finish initializing.
 *
 * Returns:
 * 0 on success, nonzero if the scanset is malformed.
 */
static int parse_scanset(const char* restrict* restrict format, FormatSpec* restrict spec) {
    if (**format == '^') {
        spec->flags |= FSF_SCANSET_NEGATED;
        ++*format;
    }
    spec->scanner = *format;
    if (**format == ']') {
        /* ']' is be included in the set if it is the first character, possibly after '^'. */
        ++*format;
    }
    while (**format) {
        if (*(*format)++ == ']') {
            return 0;
        }
    }
    return -1;
}

/*
 * If the given format string contains references to positional arguments, fills the given
 * array with `va_list` objects that can be used to access them. This only works for the
 * printf() family of functions, since the arguments to the scanf() family are all pointers.
 *
 * Returns:
 * The number of elements initialized on success, or a negative value if the format string is
 * unusable.
 */
long find_positioned_args(const char* restrict format, va_list args, va_list positioned_args[NL_ARGMAX]) {
    enum Type {
        TYPE_UNKNOWN,
        TYPE_INT,
        TYPE_DOUBLE,
        TYPE_POINTER,
        TYPE_LONG,
        TYPE_WINT,
        TYPE_LONGLONG,
        TYPE_INTMAX,
        TYPE_SIZE,
        TYPE_PTRDIFF,
        TYPE_LONGDOUBLE,
    };

    enum Type positioned_arg_types[NL_ARGMAX];
    for (size_t i = 0; i < NL_ARGMAX; ++i) {
        positioned_arg_types[i] = TYPE_UNKNOWN;
    }

    long last_positioned_argpos = 0; /* 1-based. 0 means no positioned arguments found. */
    long next_unpositioned_argpos = 0; /* POSIX says mixing positioned and unpositioned arguments is UB. We handle it anyway. */

    /* Determine the type of each positioned argument. */
    char c;
    while ((c = *format++)) {
        if (c != '%') continue;

        FormatSpec spec;
        if (parse_format_spec(&format, &spec)) continue; /* Try to continue despite the invalid format spec. */

        enum Type type = TYPE_UNKNOWN;
        switch (spec.flags & FSF_TEXT_TYPE) {
        case FSF_TEXT_INTEGER:
            switch (spec.flags & FSF_ARG_TYPE) {
            case FSF_ARG_LONG:
                type = TYPE_LONG;
                break;
            case FSF_ARG_LONG_LONG:
                type = TYPE_LONGLONG;
                break;
            case FSF_ARG_INTMAX_T:
                type = TYPE_INTMAX;
                break;
            case FSF_ARG_SIZE_T:
                type = TYPE_SIZE;
                break;
            case FSF_ARG_PTRDIFF_T:
                type = TYPE_PTRDIFF;
                break;
            default:
                type = TYPE_INT;
                break;
            }
            break;
        case FSF_TEXT_FLOAT_LOWER:
        case FSF_TEXT_FLOAT_UPPER:
        case FSF_TEXT_FLOAT_SCI_LOWER:
        case FSF_TEXT_FLOAT_SCI_UPPER:
        case FSF_TEXT_FLOAT_FLEX_LOWER:
        case FSF_TEXT_FLOAT_FLEX_UPPER:
            type = ((spec.flags & FSF_ARG_TYPE) == FSF_ARG_LONG_DOUBLE) ? TYPE_LONGDOUBLE : TYPE_DOUBLE;
            break;
        case FSF_TEXT_CHAR:
            type = ((spec.flags & FSF_ARG_TYPE) == FSF_ARG_LONG) ? TYPE_WINT : TYPE_INT;
            break;
        case FSF_TEXT_STRING:
        case FSF_TEXT_POINTER:
        case FSF_TEXT_COUNT:
            type = TYPE_POINTER;
            break;
        case FSF_TEXT_SCANSET:
        case FSF_TEXT_PERCENT:
            continue;
        }

        if (spec.argpos) {
            positioned_arg_types[spec.argpos - 1] = type;
            last_positioned_argpos = spec.argpos > last_positioned_argpos ? spec.argpos : last_positioned_argpos;
        } else {
            positioned_arg_types[next_unpositioned_argpos++] = type;
        }
    }

    if (!last_positioned_argpos) return 0;

    /* Check that we have a type for every positioned argument. */
    for (long i = 0; i < last_positioned_argpos; ++i) {
        if (positioned_arg_types[i] == TYPE_UNKNOWN) return -1;
    }

    /* Make the `va_list` object for each positioned argument. */
    va_copy(positioned_args[0], args);
    for (long i = 1; i < last_positioned_argpos; ++i) {
        va_copy(positioned_args[i], positioned_args[i - 1]);
        switch (positioned_arg_types[i - 1]) {
        case TYPE_INT:
            (void)va_arg(positioned_args[i], int);
            break;
        case TYPE_DOUBLE:
            (void)va_arg(positioned_args[i], double);
            break;
        case TYPE_POINTER:
            (void)va_arg(positioned_args[i], void*);
            break;
        case TYPE_LONG:
            (void)va_arg(positioned_args[i], long);
            break;
        case TYPE_WINT:
            (void)va_arg(positioned_args[i], wint_t);
            break;
        case TYPE_LONGLONG:
            (void)va_arg(positioned_args[i], long long);
            break;
        case TYPE_INTMAX:
            (void)va_arg(positioned_args[i], intmax_t);
            break;
        case TYPE_SIZE:
            (void)va_arg(positioned_args[i], size_t);
            break;
        case TYPE_PTRDIFF:
            (void)va_arg(positioned_args[i], ptrdiff_t);
            break;
        case TYPE_LONGDOUBLE:
            (void)va_arg(positioned_args[i], long double);
            break;
        case TYPE_UNKNOWN: /* Unreachable */
            break;
        }
    }

    return last_positioned_argpos;
}
