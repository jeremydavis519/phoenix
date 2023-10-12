/* Copyright (c) 2021-2023 Jeremy Davis (jeremydavis519@gmail.com)
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
#include <limits.h>
#include <stdarg.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <wchar.h>

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
#define FSF_TEXT_FLOAT             0x000010
#define FSF_TEXT_FLOAT_SCI_LOWER   0x000020
#define FSF_TEXT_FLOAT_SCI_UPPER   0x000030
#define FSF_TEXT_FLOAT_SHORT_LOWER 0x000040
#define FSF_TEXT_FLOAT_SHORT_UPPER 0x000050
#define FSF_TEXT_CHAR              0x000060
#define FSF_TEXT_STRING            0x000070
#define FSF_TEXT_POINTER           0x000080
#define FSF_TEXT_SCANSET           0x000090
#define FSF_TEXT_COUNT             0x0000a0
#define FSF_TEXT_PERCENT           0x0000b0

#define FSF_ARG_DEFAULT            0x000000
#define FSF_ARG_CHAR               0x000100
#define FSF_ARG_SHORT              0x000200
#define FSF_ARG_LONG               0x000300
#define FSF_ARG_LONG_LONG          0x000400
#define FSF_ARG_INTMAX_T           0x000500
#define FSF_ARG_SIZE_T             0x000600
#define FSF_ARG_PTRDIFF_T          0x000700
#define FSF_ARG_LONG_DOUBLE        0x000800

#define FSF_JUSTIFY_RIGHT          0x000000
#define FSF_JUSTIFY_LEFT           0x001000
#define FSF_FORCE_SIGN             0x002000
#define FSF_SPACE_AS_SIGN          0x004000
#define FSF_DECORATE               0x008000 /* Represents the '#' flag */
#define FSF_PAD_WITH_ZERO          0x010000
#define FSF_SCANSET_NEGATED        0x020000
#define FSF_HAS_PRECISION          0x040000
#define FSF_HAS_WIDTH              0x080000
#define FSF_PRECISION_FROM_ARG     0x100000
#define FSF_WIDTH_FROM_ARG         0x200000

typedef struct FormatSpec {
    FormatSpecFlags flags;
    size_t precision;
    size_t width;
    const char* scanner;
} FormatSpec;

typedef unsigned int CharWidth;
#define CW_UNSET    0
#define CW_NARROW   1
#define CW_WIDE     2

typedef unsigned int BufferMode;
/* The variants are _IOFBF, _IOLBF, and _IONBF. */

typedef unsigned int IOMode;
#define IO_READ    1
#define IO_WRITE   2
#define IO_RW      IO_READ | IO_WRITE

struct mbstate_t {
    /* TODO */
};

/* FIXME: This has to be defined in stdio.h, or else it'll be an incomplete type in client code. */
struct fpos_t {
    off_t     offset;         /* Number of bytes into the file */
    mbstate_t mb_parse_state; /* State of the multibyte character parser */
};

struct FILE {
    int          is_open         : 1;
    CharWidth    char_width      : 2;
    BufferMode   buffer_mode     : 2;
    IOMode       io_mode         : 2;
    int          eof             : 1;
    int          error           : 1;
    int          malloced_buffer : 1;
    int          fildes;       /* File descriptor */
    fpos_t       position;
    off_t        length;
    char*        buffer;       /* Pointer to buffer being used, or NULL */
    size_t       buffer_size;
    size_t       buffer_index; /* Index of next byte to set in the buffer */
    atomic_flag  lock;
    union {
        wint_t wc;
        char   c[sizeof(wint_t)];
    }            pushback_buffer;
    uint8_t      pushback_index;
};

static size_t fread_locked(void* restrict buffer, size_t size, size_t count, FILE* restrict stream);
static int fflush_locked(FILE* stream);
static int fseek_locked(FILE* stream, long offset, int whence);
static int fseeko_locked(FILE* stream, off_t offset, int whence);

static int parse_format_spec(const char* restrict* restrict format, FormatSpec* restrict spec);
static int parse_scanset(const char* restrict* restrict format, FormatSpec* restrict spec);
static bool try_lock_file(FILE* stream);
static void lock_file(FILE* stream);
static void unlock_file(FILE* stream);

static FILE files[FOPEN_MAX] = {0};


/* Standard input and output */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/stdin.html */
FILE* stdin  = &files[0];
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/stdout.html */
FILE* stdout = &files[1];
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/stderr.html */
FILE* stderr = &files[2];


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

    lock_file(stream);
    int result = fflush_locked(stream);
    unlock_file(stream);
    return result;
}

int fflush_locked(FILE* stream) {
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

        if (i == FOPEN_MAX) { /* NB: STREAM_MAX is defined to be equal to FOPEN_MAX. */
            errno = EMFILE;
            return NULL;
        }

        /* Try to claim it before another thread does. */
        if (try_lock_file(files + i)) break;
    }

    /* Open the file. */
    FILE* result = freopen(path, mode, files + i);

    unlock_file(files + i);
    return result;
}

FILE* freopen(const char* restrict path, const char* restrict mode, FILE* restrict stream); */

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

    lock_file(stream);

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

    unlock_file(stream);

    return 0;
}


/* Formatted input/output */
/* TODO
int dprintf(int fildes, const char* restrict format, ...); */

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

/* TODO
int vfprintf(FILE* restrict stream, const char* restrict format, va_list args);
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

/* TODO
int vsnprintf(char* restrict s, size_t n, const char* restrict format, va_list args);
int vsprintf(char* restrict s, const char* restrict format, va_list args); */

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/vsscanf.html */
int vsscanf(const char* restrict s, const char* restrict format, va_list args) {
    int sign;
    int radix, dec_digits, hex_digits;
    uintmax_t i_acc;
    void* out;

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
        if (parse_format_spec(&format, &spec)) {
            goto matching_break;
        }

        size_t width_counter = SIZE_MAX;
        if (spec.flags & FSF_HAS_WIDTH) {
            if (spec.flags & FSF_WIDTH_FROM_ARG) {
                width_counter = va_arg(args, int);
            } else {
                width_counter = spec.width;
            }
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
                goto matching_break;
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
                switch (spec.flags & FSF_ARG_TYPE) {
                case FSF_ARG_DEFAULT:
                    if ((spec.flags & FSF_TEXT_TYPE) == FSF_TEXT_POINTER) {
                        *va_arg(args, void**) = (void*)(sign * (intmax_t)i_acc);
                    } else if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *va_arg(args, int*) = sign * (intmax_t)i_acc;
                    } else {
                        *va_arg(args, unsigned int*) = sign * i_acc;
                    }
                    break;
                case FSF_ARG_CHAR:
                    if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *va_arg(args, signed char*) = sign * (intmax_t)i_acc;
                    } else {
                        *va_arg(args, unsigned char*) = sign * i_acc;
                    }
                    break;
                case FSF_ARG_SHORT:
                    if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *va_arg(args, short int*) = sign * (intmax_t)i_acc;
                    } else {
                        *va_arg(args, unsigned short int*) = sign * i_acc;
                    }
                    break;
                case FSF_ARG_LONG:
                    if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *va_arg(args, long int*) = sign * (intmax_t)i_acc;
                    } else {
                        *va_arg(args, unsigned long int*) = sign * i_acc;
                    }
                    break;
                case FSF_ARG_LONG_LONG:
                    if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *va_arg(args, long long int*) = sign * (intmax_t)i_acc;
                    } else {
                        *va_arg(args, unsigned long int*) = sign * i_acc;
                    }
                    break;
                case FSF_ARG_INTMAX_T:
                    if ((spec.flags & FSF_SIGN) == FSF_SIGNED) {
                        *va_arg(args, intmax_t*) = sign * (intmax_t)i_acc;
                    } else {
                        *va_arg(args, uintmax_t*) = sign * i_acc;
                    }
                    break;
                case FSF_ARG_SIZE_T:
                    *va_arg(args, size_t*) = sign * i_acc;
                    break;
                case FSF_ARG_PTRDIFF_T:
                    *va_arg(args, ptrdiff_t*) = sign * (intmax_t)i_acc;
                    break;

                default:
                    goto matching_break;
                }
                ++args_filled;
            }
            break;
        case FSF_TEXT_FLOAT:
        case FSF_TEXT_FLOAT_SCI_LOWER:
        case FSF_TEXT_FLOAT_SCI_UPPER:
        case FSF_TEXT_FLOAT_SHORT_LOWER:
        case FSF_TEXT_FLOAT_SHORT_UPPER:
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
            va_arg(args, void*); /* Just to avoid really weird undefined behavior if this path is taken */
            ++args_filled; /* TODO: Should be done only if the argument is actually used */
            break;
        case FSF_TEXT_CHAR:
            if (!(spec.flags & FSF_HAS_WIDTH)) {
                width_counter = 1;
            }
            if (store_arg) {
                out = va_arg(args, void*);
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
                out = va_arg(args, void*);
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
            out = store_arg ? va_arg(args, void*) : NULL;
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
                switch (spec.flags & FSF_ARG_TYPE) {
                case FSF_ARG_DEFAULT:
                    *va_arg(args, int*) = s - s_start;
                    break;
                case FSF_ARG_CHAR:
                    *va_arg(args, signed char*) = s - s_start;
                    break;
                case FSF_ARG_SHORT:
                    *va_arg(args, short int*) = s - s_start;
                    break;
                case FSF_ARG_LONG:
                    *va_arg(args, long int*) = s - s_start;
                    break;
                case FSF_ARG_LONG_LONG:
                    *va_arg(args, long long int*) = s - s_start;
                    break;
                case FSF_ARG_INTMAX_T:
                    *va_arg(args, intmax_t*) = s - s_start;
                    break;
                case FSF_ARG_SIZE_T:
                    *va_arg(args, size_t*) = s - s_start;
                    break;
                case FSF_ARG_PTRDIFF_T:
                    *va_arg(args, ptrdiff_t*) = s - s_start;
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

static int fgetc_locked(FILE* stream) {
    unsigned char buf[1];
    if (fread_locked(&buf, 1, 1, stream) < 1) {
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

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fgets.html */
char* fgets(char* restrict str, int num, FILE* restrict stream) {
    if (num == 0) {
        /* No room in the buffer for even the null terminator */
        errno = EINVAL;
        return NULL;
    }

    lock_file(stream);

    char* result = str;

    /* "If the end-of-file condition is encountered before any bytes are read, the contents of the array pointed to by s shall not be changed." */
    if (stream->eof) return NULL;

    while (--num) {
        if (stream->eof) break;
        int c = fgetc_locked(stream);
        if (c == EOF) {
            if (stream->error) result = NULL;
            break;
        }
        *str++ = (char)c;
        if (c == '\n') break;
    }
    *str = '\0';

    unlock_file(stream);
    return result;
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

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fputs.html */
int fputs(const char* str, FILE* stream) {
    lock_file(stream);

    if (!stream->buffer_size || stream->buffer_mode == _IONBF) {
        /* The stream is unbuffered, so just send the string. */
        /* TODO: Implement this, ideally by sending the whole string at once. */
        unlock_file(stream);
        stream->error = -1;
        return EOF;
    }

    while (*str) {
        /* Buffer as many characters as possible. */
        while (stream->buffer_index < stream->buffer_size) {
            stream->buffer[stream->buffer_index++] = *str++;
            if (!*str) goto done;
        }

        /* Flush the buffer before continuing. */
        if (fflush_locked(stream)) {
            unlock_file(stream);
            return EOF;
        }
    }

done:
    unlock_file(stream);
    return 0;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/putchar.html */
int putchar(int ch) {
    return fputc(ch, stdout);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/puts.html */
int puts(const char* str) {
    if (fputs(str, stdout) < 0) return EOF;
    return putchar('\n');
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ungetc.html */
int ungetc(int ch, FILE* stream) {
    if (ch == EOF) return EOF; /* Ungetting nothing */

    lock_file(stream);
    int result = EOF;
    if (stream->pushback_index == sizeof(stream->pushback_buffer.c)) goto end; /* Buffer already full */

    stream->pushback_buffer.c[stream->pushback_index++] = ch;
    stream->eof = false;
    result = ch;

end:
    unlock_file(stream);
    return result;
}

/* Direct input/output */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fread.html */
size_t fread(void* restrict buffer, size_t size, size_t count, FILE* restrict stream) {
    if (size == 0 || count == 0) return 0;

    lock_file(stream);
    size_t result = fread_locked(buffer, size, count, stream);
    unlock_file(stream);
    return result;
}

size_t fread_locked(void* restrict buffer, size_t size, size_t count, FILE* restrict stream) {
    if (size == 0 || count == 0) return 0;
    if (!stream->is_open || !(stream->io_mode & IO_READ)) {
        stream->error = -1;
        errno = EBADF;
        return 0;
    }
    if (stream->eof) return 0;

    size_t bytes_read = 0;
    size_t total_size = size * count;

    size_t pushback_bytes = stream->pushback_index < total_size ? stream->pushback_index : total_size;
    memcpy(buffer, stream->pushback_buffer.c, pushback_bytes);
    bytes_read += pushback_bytes;
    stream->pushback_index -= pushback_bytes;

    bytes_read += read(stream->fildes, buffer, total_size - bytes_read);

    return bytes_read / size;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fwrite.html */
size_t fwrite(const void* restrict buffer, size_t size, size_t count, FILE* restrict stream) {
    if (size == 0 || count == 0) return 0;

    lock_file(stream);

    size_t bytes_remaining = size * count;
    size_t obj_buffer_index = 0;
    while (bytes_remaining) {
        if (!stream->buffer_size || stream->buffer_mode == _IONBF) {
            /* The stream is unbuffered, so just send one character at a time. */
            /* TODO */
            stream->error = -1;
            break;
        }

        /* Flush the buffer if it's full. */
        if (stream->buffer_index >= stream->buffer_size && fflush_locked(stream)) {
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

    unlock_file(stream);
    return (size * count - bytes_remaining) / size;
}


/* File positioning */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fgetpos.html */
int fgetpos(FILE* restrict stream, fpos_t* restrict pos) {
    lock_file(stream);

    /* FIXME: If "The file descriptor underlying stream is not valid." */
    if (false) {
        unlock_file(stream);
        errno = EBADF;
        return -1;
    }

    /* FIXME: If "The file descriptor underlying stream is associated with a pipe, FIFO, or socket." */
    if (false) {
        unlock_file(stream);
        errno = ESPIPE;
        return -1;
    }

    *pos = stream->position;

    /* Correct the position for buffered writes and calls to unget. */
    pos->offset += stream->buffer_index;
    pos->offset -= stream->pushback_index;

    unlock_file(stream);
    return 0;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fsetpos.html */
int fsetpos(FILE* stream, const fpos_t* pos) {
    lock_file(stream);

    if (fflush_locked(stream)) {
        unlock_file(stream);
        return -1;
    }

    stream->position = *pos;
    stream->pushback_buffer.wc = WEOF;
    stream->pushback_index = 0;
    stream->eof = false;

    unlock_file(stream);
    return 0;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fseek.html */
int fseek(FILE* stream, long offset, int whence) {
    lock_file(stream);
    int result = fseek_locked(stream, offset, whence);
    unlock_file(stream);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/fseeko.html */
int fseeko(FILE* stream, off_t offset, int whence) {
    lock_file(stream);
    int result = fseeko_locked(stream, offset, whence);
    unlock_file(stream);
    return result;
}

#define FSEEK_GENERIC(fn_name, offset_t, offset_t_max, offset_t_min) \
static int fn_name(FILE* stream, offset_t offset, int whence) { \
    /* FIXME: If "The file descriptor underlying `stream` is associated with a pipe, FIFO, or socket." */ \
    if (false) { \
        errno = ESPIPE; \
        return -1; \
    } \
\
    if (fflush_locked(stream)) { \
        return -1; \
    } \
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
            errno = EOVERFLOW; \
            return -1; \
        } \
        stream->position.offset += offset; \
        break; \
    case SEEK_END: \
        if ( \
            (stream->length <= LONG_MAX && offset > offset_t_max - (long)stream->length) || \
            stream->length + offset > offset_t_max \
        ) { \
            /* Seeking beyond the range of a `long` */ \
            errno = EOVERFLOW; \
            return -1; \
        } \
        stream->position.offset = stream->length + offset; \
        break; \
    default: \
        /* Unrecognized seek origin */ \
        errno = EINVAL; \
        return -1; \
    } \
\
    stream->pushback_buffer.wc = WEOF; \
    stream->pushback_index = 0; \
    stream->eof = false; \
\
    return 0; \
}

FSEEK_GENERIC(fseek_locked, long, LONG_MAX, LONG_MIN)
FSEEK_GENERIC(fseeko_locked, off_t, OFF_MAX, OFF_MIN)

#define FTELL_GENERIC(fn_name, offset_t, offset_t_max) \
offset_t fn_name(FILE* stream) { \
    lock_file(stream); \
\
    /* FIXME: If "The file descriptor underlying `stream` is not an open file descriptor." */ \
    if (false) { \
        errno = EBADF; \
        return -1L; \
    } \
\
    /* FIXME: If "The file descriptor underlying `stream` is associated with a pipe, FIFO, or socket." */ \
    if (false) { \
        errno = ESPIPE; \
        return -1L; \
    } \
\
    /* Calculate current offset, including buffered writes and calls to unget. */ \
    offset_t offset = stream->position.offset + stream->buffer_index - stream->pushback_index; \
\
    if (offset > offset_t_max) { \
        errno = EOVERFLOW; \
        return -1L; \
    } \
\
    unlock_file(stream); \
    return offset; \
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ftell.html */
FTELL_GENERIC(ftell, long, LONG_MAX)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ftello.html */
FTELL_GENERIC(ftello, off_t, OFF_MAX)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/rewind.html */
void rewind(FILE* stream) {
    lock_file(stream);

    (void)fseek_locked(stream, 0L, SEEK_SET);
    stream->error = false;

    unlock_file(stream);
}


/* Error-handling */
/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/clearerr.html */
void clearerr(FILE* stream) {
    lock_file(stream);
    stream->eof = false;
    stream->error = false;
    unlock_file(stream);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/feof.html */
int feof(FILE* stream) {
    lock_file(stream);
    int result = stream->eof;
    unlock_file(stream);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ferror.html */
int ferror(FILE* stream) {
    lock_file(stream);
    int result = stream->error;
    unlock_file(stream);
    return result;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/perror.html */
void perror(const char* s) {
    switch (stderr->char_width) {
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
}


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
 * 0 on success, nonzero if the format string is malformed.
 */
static int parse_format_spec(const char* restrict* restrict format, FormatSpec* restrict spec) {
    spec->flags = 0;

    /* Flags */
    for (; **format; ++*format) {
        switch (**format) {
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
    } else if (**format >= '0' && **format <= '9') {
        spec->flags |= FSF_HAS_WIDTH;
        spec->width = 0;
        do {
            spec->width = 10 * spec->width + (*(*format++) - '0');
        } while (**format >= '0' && **format <= '9');
    }

    /* Precision */
    if (**format == '.') {
        spec->flags |= FSF_HAS_PRECISION;
        if (*(++*format) == '*') {
            spec->flags |= FSF_PRECISION_FROM_ARG;
        } else {
            spec->precision = 0;
            while (**format >= '0' && **format <= '9') {
                spec->precision = 10 * spec->precision + (*(*format++) - '0');
            }
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
    switch (*(*format++)) {
    case 'i':
        spec->flags |= FSF_SIGNED | FSF_ANY_RADIX | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 1;
        }
        return 0;
    case 'u':
        spec->flags |= FSF_UNSIGNED | FSF_DECIMAL | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 1;
        }
        return 0;
    case 'd':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 1;
        }
        return 0;
    case 'o':
        spec->flags |= FSF_UNSIGNED | FSF_OCTAL | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 1;
        }
        return 0;
    case 'x':
        spec->flags |= FSF_UNSIGNED | FSF_HEX_LOWER | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 1;
        }
        return 0;
    case 'X':
        spec->flags |= FSF_UNSIGNED | FSF_HEX_UPPER | FSF_TEXT_INTEGER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 1;
        }
        return 0;
    case 'f':
    case 'F':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 6;
        }
        return 0;
    case 'e':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT_SCI_LOWER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 6;
        }
        return 0;
    case 'E':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT_SCI_UPPER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 6;
        }
        return 0;
    case 'g':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT_SHORT_LOWER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 6;
        }
        return 0;
    case 'G':
        spec->flags |= FSF_SIGNED | FSF_DECIMAL | FSF_TEXT_FLOAT_SHORT_UPPER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 6;
        }
        return 0;
    case 'a':
        spec->flags |= FSF_SIGNED | FSF_HEX_LOWER | FSF_TEXT_FLOAT_SCI_LOWER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 6;
        }
        return 0;
    case 'A':
        spec->flags |= FSF_SIGNED | FSF_HEX_UPPER | FSF_TEXT_FLOAT_SCI_UPPER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 6;
        }
        return 0;
    case 'c':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_CHAR;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 1;
        }
        return 0;
    case 's':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_STRING;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = SIZE_MAX;
        }
        return 0;
    case 'p':
        spec->flags |= FSF_UNSIGNED | FSF_HEX_LOWER | FSF_TEXT_POINTER;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 1;
        }
        return 0;
    case '[':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_SCANSET;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 1;
        }
        return parse_scanset(format, spec);
    case 'n':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_COUNT;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 0;
        }
        return 0;
    case '%':
        spec->flags |= FSF_UNSIGNED | FSF_TEXT_PERCENT;
        if (!(spec->flags & FSF_HAS_PRECISION)) {
            spec->precision = 1;
        }
        return 0;
    
    default:
        /* Invalid format specification */
        return 1;
    }
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
        if (*(*format++) == ']') {
            return 0;
        }
    }
    return 1;
}

/*
 * Tries once to lock a file.
 *
 * Parameters:
 * `stream`: The file to try to lock.
 *
 * Returns:
 * `true` if the attempt was successful; `false` if the file was already locked.
 */
static bool try_lock_file(FILE* stream) {
    return !atomic_flag_test_and_set_explicit(&stream->lock, memory_order_acq_rel);
}

/*
 * Locks a file to avoid data races when accessing its contents and position. Blocks if necessary.
 *
 * Parameters:
 * `stream`: The file to lock.
 */
static void lock_file(FILE* stream) {
    while (atomic_flag_test_and_set_explicit(&stream->lock, memory_order_acq_rel)) {
        sleep(0);
    }
}

/*
 * Unlocks a previously locked file.
 *
 * Parameters:
 * `stream`: The file to unlock.
 */
static void unlock_file(FILE* stream) {
    atomic_flag_clear_explicit(&stream->lock, memory_order_release);
}
