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

#include <ctype.h>
#include <limits.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

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

static int parse_format_spec(const char** format, FormatSpec* spec);
static int parse_scanset(const char** format, FormatSpec* spec);

/* Operations on files */
/* TODO 
int remove(const char* filename);
int rename(const char* oldname, const char* newname);
FILE* tmpfile(void);
char* tmpnam(char* str); */


/* File access */
/* TODO 
int fclose(FILE* stream);
int fflush(FILE* stream);
FILE* fopen(const char* filename, const char* mode);
FILE* freopen(const char* filename, const char* mode, FILE* stream);
void setbuf(FILE* stream, char* buffer);
int setvbuf(FILE* stream, char* buffer, int mode, size_t size); */


/* Formatted input/output */
/* TODO 
int fprintf(FILE* stream, const char* format, ...);
int fscanf(FILE* stream, const char* format, ...); */

int printf(const char* format, ...) {
    /* FIXME: This printf implementation is far from complete. */
    puts(format);
    return strlen(format);
}

/* TODO
int scanf(const char* format, ...);
int snprintf(char* s, size_t n, const char* format, ...);
int sprintf(char* s, const char* format, ...); */

int sscanf(const char* s, const char* format, ...) {
    va_list args;
    va_start(args, format);
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
        FormatSpec spec;
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
            if (width_counter != 0) {
                sign = 1;
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
            if (store_arg) {
                out = va_arg(args, void*);
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
    va_end(args);
    return args_filled; /* TODO: If EOF is reached before any data can be read, return EOF. */
}

/* TODO
int vfprintf(FILE* stream, const char* format, va_list arg);
int vfscanf(FILE* stream, const char* format, va_list arg);
int vprintf(const char* format, va_list arg);
int vscanf(const char* format, va_list arg);
int vsnprintf(char* s, size_t n, const char* format, va_list arg);
int vsprintf(char* s, const char* format, va_list arg);
int vsscanf(const char* s, const char* format, va_list arg); */


/* Character input/output */
/* TODO 
int fgetc(FILE* stream);
char* fgets(char* str, int num, FILE* stream);
int fputc(int character, FILE* stream);
int fputs(const char* str, FILE* stream);
int getc(FILE* stream);
int getchar(void);
int putc(int character, FILE* stream); */

int putchar(int c) {
    /* FIXME: This system call is temporary. */
    register int c2 asm ("x2") = c;
    asm volatile("svc 0xff00" :: "r"(c2));
    return c;
}

int puts(const char* str) {
    while (*str) {
        putchar(*str++);
    }
    putchar('\n'); /* This is in the standard for puts, even though it's not for fputs. */
    return 0; /* TODO: Return EOF on failure. */
}

/* TODO
int ungetc(int character, FILE* stream); */

/* Direct input/output */
/* TODO 
size_t fread(void* ptr, size_t size, size_t count, FILE* stream);
size_t fwrite(const void* ptr, size_t size, size_t count, FILE* stream); */


/* File positioning */
/* TODO 
int fgetpos(FILE* stream, fpos_t* pos);
int fseek(FILE* stream, long int offset, int origin);
long int ftell(FILE* stream);
void rewind(FILE* stream); */


/* Error-handling */
/* TODO 
void clearerr(FILE* stream);
int feof(FILE* stream);
int ferror(FILE* stream);
void perror(FILE* stream); */

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
static int parse_format_spec(const char** format, FormatSpec* spec) {
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
static int parse_scanset(const char** format, FormatSpec* spec) {
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
