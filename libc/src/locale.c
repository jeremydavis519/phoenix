/* Copyright (c) 2023-2024 Jeremy Davis (jeremydavis519@gmail.com)
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
#include <locale.h>
#include <stdlib.h>
#include <string.h>

/* Character class definitions */
struct ctype {
    size_t upper_len;
    const char* upper;
    size_t lower_len;
    const char* lower;
    size_t alpha_len;
    const char* alpha;
    size_t digit_len;
    const char* digit;
    size_t space_len;
    const char* space;
    size_t cntrl_len;
    const char* cntrl;
    size_t punct_len;
    const char* punct;
    size_t graph_len;
    const char* graph;
    size_t print_len;
    const char* print;
    size_t xdigit_len;
    const char* xdigit;
    size_t blank_len;
    const char* blank;
    size_t toupper_len;
    const char* toupper_from;
    const char* toupper_to;
    size_t tolower_len;
    const char* tolower_from;
    const char* tolower_to;
};

/* A single collation element and its weight */
struct collation_weight {
    const char* elem; /* Special case: Strings starting with '\0' are terminated by the second '\0', not the first. Strings here can't be empty. */
    uint64_t    weight;
};

/* Information about the representation of an era (AD, BC, etc.) */
struct era {
    char        direction;
    uint32_t    offset;
    int32_t     start_date_year;
    int32_t     end_date_year; /* If end_date_year is on the wrong side of start_date_year according to direction, there is no end date. */
    uint8_t     start_date_month;
    uint8_t     start_date_day;
    uint8_t     end_date_month;
    uint8_t     end_date_day;
    const char* name;
    const char* format;
};

/* Textual representation of time and date */
struct time {
    const char* d_t_fmt;
    const char* d_fmt;
    const char* t_fmt;
    const char* am;
    const char* pm;
    const char* t_fmt_ampm;
    const char* day[7];
    const char* abday[7];
    const char* mon[12];
    const char* abmon[12];
    const struct era* eras; /* Terminated by an era with direction = '\0' */
    const char* era_d_fmt;
    const char* era_t_fmt;
    const char* era_d_t_fmt;
    const char* alt_digits;
};

/* Regular expressions for recognizing affirmative and negative responses */
struct messages {
    const char* yesexpr;
    const char* noexpr;
};

/* Built-in locales */
#define BUILTIN_LOCALES_COUNT 1
#define MAX_LOCALE_NAME_LEN 1
static const size_t POSIX_LOCALE_INDEX = 0; /* "POSIX" is equivalent to "C". */
static char* const BUILTIN_LOCALE_NAMES[BUILTIN_LOCALES_COUNT] = {"C"};
static struct lconv BUILTIN_LOCALE_CONVS[BUILTIN_LOCALES_COUNT] = {
    {
        .decimal_point      = ".",
        .grouping           = "",
        .thousands_sep      = "",

        .int_curr_symbol    = "",
        .currency_symbol    = "",
        .mon_decimal_point  = "",
        .mon_thousands_sep  = "",
        .mon_grouping       = "",
        .positive_sign      = "",
        .negative_sign      = "",
        .int_frac_digits    = CHAR_MAX,
        .frac_digits        = CHAR_MAX,
        .int_p_cs_precedes  = CHAR_MAX,
        .int_p_sep_by_space = CHAR_MAX,
        .int_p_sign_posn    = CHAR_MAX,
        .int_n_cs_precedes  = CHAR_MAX,
        .int_n_sep_by_space = CHAR_MAX,
        .int_n_sign_posn    = CHAR_MAX,
        .p_cs_precedes      = CHAR_MAX,
        .p_sep_by_space     = CHAR_MAX,
        .p_sign_posn        = CHAR_MAX,
        .n_cs_precedes      = CHAR_MAX,
        .n_sep_by_space     = CHAR_MAX,
        .n_sign_posn        = CHAR_MAX
    }
};
static const struct ctype BUILTIN_LOCALE_CTYPES[BUILTIN_LOCALES_COUNT] = {
    {
        .upper_len = 26,
        .upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        .lower_len = 26,
        .lower = "abcdefghijklmnopqrstuvwxyz",
        .alpha_len = 52,
        .alpha = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        .digit_len = 10,
        .digit = "0123456789",
        .space_len = 6,
        .space = "\t\n\v\f\r ",
        .cntrl_len = 33,
        .cntrl = "\a\b\t\n\v\f\r\0\x01\x02\x03\x04\x05\x06\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x7f",
        .punct_len = 32,
        .punct = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~",
        .graph_len = 94,
        .graph = "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~",
        .print_len = 95,
        .print = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~",
        .xdigit_len = 22,
        .xdigit = "0123456789ABCDEFabcdef",
        .blank_len = 2,
        .blank = " \t",
        .toupper_len = 26,
        .toupper_from = "abcdefghijklmnopqrstuvwxyz",
        .toupper_to   = "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        .tolower_len = 26,
        .tolower_from = "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        .tolower_to   = "abcdefghijklmnopqrstuvwxyz"
    }
};
static const struct collation_weight POSIX_LOCALE_COLLATIONS[129] = {
    {"\0", 0}, {"\x01", 1}, {"\x02", 2}, {"\x03", 3}, {"\x04", 4}, {"\x05", 5}, {"\x06", 6}, {"\a", 7}, {"\b", 8}, {"\t", 9},
    {"\n", 10}, {"\v", 11}, {"\f", 12}, {"\r", 13}, {"\x0e", 14}, {"\x0f", 15}, {"\x10", 16}, {"\x11", 17}, {"\x12", 18}, {"\x13", 19},
    {"\x14", 20}, {"\x15", 21}, {"\x16", 22}, {"\x17", 23}, {"\x18", 24}, {"\x19", 25}, {"\x1a", 26}, {"\x1b", 27}, {"\x1c", 28},
    {"\x1d", 29}, {"\x1e", 30}, {"\x1f", 31}, {" ", 32}, {"!", 33}, {"\"", 34}, {"#", 35}, {"$", 36}, {"%", 37}, {"&", 38}, {"'", 39},
    {"(", 40}, {")", 41}, {"*", 42}, {"+", 43}, {",", 44}, {"-", 45}, {".", 46}, {"/", 47}, {"0", 48}, {"1", 49}, {"2", 50}, {"3", 51},
    {"4", 52}, {"5", 53}, {"6", 54}, {"7", 55}, {"8", 56}, {"9", 57}, {":", 58}, {";", 59}, {"<", 60}, {"=", 61}, {">", 62}, {"?", 63},
    {"@", 64}, {"A", 65}, {"B", 66}, {"C", 67}, {"D", 68}, {"E", 69}, {"F", 70}, {"G", 71}, {"H", 72}, {"I", 73}, {"J", 74}, {"K", 75},
    {"L", 76}, {"M", 77}, {"N", 78}, {"O", 79}, {"P", 80}, {"Q", 81}, {"R", 82}, {"S", 83}, {"T", 84}, {"U", 85}, {"V", 86}, {"W", 87},
    {"X", 88}, {"Y", 89}, {"Z", 90}, {"[", 91}, {"\\", 92}, {"]", 93}, {"^", 94}, {"_", 95}, {"`", 96}, {"a", 97}, {"b", 98}, {"c", 99},
    {"d", 100}, {"e", 101}, {"f", 102}, {"g", 103}, {"h", 104}, {"i", 105}, {"j", 106}, {"k", 107}, {"l", 108}, {"m", 109}, {"n", 110},
    {"o", 111}, {"p", 112}, {"q", 113}, {"r", 114}, {"s", 115}, {"t", 116}, {"u", 117}, {"v", 118}, {"w", 119}, {"x", 120}, {"y", 121},
    {"z", 122}, {"{", 123}, {"|", 124}, {"}", 125}, {"~", 126}, {"\x7f", 127}, {NULL}
};
static const struct collation_weight* const BUILTIN_LOCALE_COLLATIONS[BUILTIN_LOCALES_COUNT] = {
    POSIX_LOCALE_COLLATIONS
};
static const struct era NO_ERA = {.direction = '\0'};
static const struct time BUILTIN_LOCALE_TIMES[BUILTIN_LOCALES_COUNT] = {
    {
        .d_t_fmt = "%a %b %e %H:%M:%S %Y",
        .d_fmt = "%m/%d/%y",
        .t_fmt = "%H:%M:%S",
        .am = "AM",
        .pm = "PM",
        .t_fmt_ampm = "%I:%M:%S %p",
        .day = {"Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"},
        .abday = {"Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"},
        .mon = {"January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"},
        .abmon = {"Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"},
        .eras = &NO_ERA,
        .era_d_fmt = "",
        .era_t_fmt = "",
        .era_d_t_fmt = "",
        .alt_digits = ""
    }
};
static const struct messages BUILTIN_LOCALE_MESSAGES[BUILTIN_LOCALES_COUNT] = {
    {
        .yesexpr = "^[yY]",
        .noexpr = "^[nN]"
    }
};

struct locale {
    size_t collate;
    size_t ctype;
    size_t messages;
    size_t monetary;
    size_t numeric;
    size_t time;
};

static struct locale default_locale = {.collate = 0, .ctype = 0, .messages = 0, .monetary = 0, .numeric = 0, .time = 0};
static struct locale global_locale = {.collate = 0, .ctype = 0, .messages = 0, .monetary = 0, .numeric = 0, .time = 0};
const locale_t LC_GLOBAL_LOCALE = &global_locale;
static __thread struct locale* current_locale = &global_locale;

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/duplocale.html */
locale_t duplocale(locale_t orig) {
    struct locale* dup = (struct locale*)malloc(sizeof(struct locale));
    if (!dup) return NULL;
    memcpy(dup, (struct locale*)orig, sizeof(struct locale));
    return dup;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/freelocale.html */
void freelocale(locale_t locale) {
    free(locale);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/localeconv.html */
struct lconv* localeconv(void) {
    static struct lconv conv;

    struct lconv* numeric = &BUILTIN_LOCALE_CONVS[current_locale->numeric];
    struct lconv* monetary = &BUILTIN_LOCALE_CONVS[current_locale->monetary];

    conv.decimal_point = numeric->decimal_point;
    conv.grouping = numeric->grouping;
    conv.thousands_sep = numeric->thousands_sep;

    conv.int_curr_symbol = monetary->int_curr_symbol;
    conv.currency_symbol = monetary->currency_symbol;
    conv.mon_decimal_point = monetary->mon_decimal_point;
    conv.mon_thousands_sep = monetary->mon_thousands_sep;
    conv.mon_grouping = monetary->mon_grouping;
    conv.positive_sign = monetary->positive_sign;
    conv.negative_sign = monetary->negative_sign;
    conv.int_frac_digits = monetary->int_frac_digits;
    conv.frac_digits = monetary->frac_digits;
    conv.int_p_cs_precedes = monetary->int_p_cs_precedes;
    conv.int_p_sep_by_space = monetary->int_p_sep_by_space;
    conv.int_p_sign_posn = monetary->int_p_sign_posn;
    conv.int_n_cs_precedes = monetary->int_n_cs_precedes;
    conv.int_n_sep_by_space = monetary->int_n_sep_by_space;
    conv.int_n_sign_posn = monetary->int_n_sign_posn;
    conv.p_cs_precedes = monetary->p_cs_precedes;
    conv.p_sep_by_space = monetary->p_sep_by_space;
    conv.p_sign_posn = monetary->p_sign_posn;
    conv.n_cs_precedes = monetary->n_cs_precedes;
    conv.n_sep_by_space = monetary->n_sep_by_space;
    conv.n_sign_posn = monetary->n_sign_posn;

    return &conv;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/newlocale.html */
locale_t newlocale(int category_mask, const char* locale, locale_t base) {
    if ((category_mask & ~LC_ALL_MASK) || !locale) {
        errno = EINVAL;
        return NULL;
    }

    if (locale[0] == '\0') {
        /* TODO: Get the locale from environment variables. */
    }

    size_t locale_index = 0;
    if (!strcmp(locale, "POSIX")) {
        locale_index = POSIX_LOCALE_INDEX;
    } else {
        do {
            if (locale_index >= BUILTIN_LOCALES_COUNT) {
                errno = EINVAL;
                return NULL;
            }
        } while (strcmp(locale, BUILTIN_LOCALE_NAMES[locale_index++]));
        --locale_index;
    }

    struct locale* b = (struct locale*)base;
    if (!b && !(b = duplocale(&default_locale))) return NULL;

    if (category_mask & LC_COLLATE_MASK) b->collate = locale_index;
    if (category_mask & LC_CTYPE_MASK) b->ctype = locale_index;
    if (category_mask & LC_MESSAGES_MASK) b->messages = locale_index;
    if (category_mask & LC_MONETARY_MASK) b->monetary = locale_index;
    if (category_mask & LC_NUMERIC_MASK) b->numeric = locale_index;
    if (category_mask & LC_TIME_MASK) b->time = locale_index;

    return base;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/setlocale.html */
char* setlocale(int category, const char* locale) {
    if (!locale) {
        /* Query current global locale */
        static char buf[92 + 6 * MAX_LOCALE_NAME_LEN];
        switch (category) {
            case LC_COLLATE:  return BUILTIN_LOCALE_NAMES[global_locale.collate];
            case LC_CTYPE:    return BUILTIN_LOCALE_NAMES[global_locale.ctype];
            case LC_MESSAGES: return BUILTIN_LOCALE_NAMES[global_locale.messages];
            case LC_MONETARY: return BUILTIN_LOCALE_NAMES[global_locale.monetary];
            case LC_NUMERIC:  return BUILTIN_LOCALE_NAMES[global_locale.numeric];
            case LC_TIME:     return BUILTIN_LOCALE_NAMES[global_locale.time];
            case LC_ALL:
                sprintf(buf, "LC_COLLATE: \"%s\", LC_CTYPE: \"%s\", LC_MESSAGES: \"%s\", LC_MONETARY: \"%s\", LC_NUMERIC: \"%s\", LC_TIME: \"%s\"",
                    BUILTIN_LOCALE_NAMES[global_locale.collate], BUILTIN_LOCALE_NAMES[global_locale.ctype],
                    BUILTIN_LOCALE_NAMES[global_locale.messages], BUILTIN_LOCALE_NAMES[global_locale.monetary],
                    BUILTIN_LOCALE_NAMES[global_locale.numeric], BUILTIN_LOCALE_NAMES[global_locale.time]);
                return buf;

            default:
                return NULL;
        }
    }

    if (locale[0] == '\0') {
        /* TODO: Get the locale from environment variables. */
    }

    size_t locale_index = 0;
    if (!strcmp(locale, "POSIX")) {
        locale_index = POSIX_LOCALE_INDEX;
    } else {
        do {
            if (locale_index >= BUILTIN_LOCALES_COUNT) return NULL;
        } while (strcmp(locale, BUILTIN_LOCALE_NAMES[locale_index++]));
        --locale_index;
    }

    /* Set global locale */
    switch (category) {
        case LC_COLLATE:
            global_locale.collate = locale_index;
            return BUILTIN_LOCALE_NAMES[locale_index];
        case LC_CTYPE:
            global_locale.ctype = locale_index;
            return BUILTIN_LOCALE_NAMES[locale_index];
        case LC_MESSAGES:
            global_locale.messages = locale_index;
            return BUILTIN_LOCALE_NAMES[locale_index];
        case LC_MONETARY:
            global_locale.monetary = locale_index;
            return BUILTIN_LOCALE_NAMES[locale_index];
        case LC_NUMERIC:
            global_locale.numeric = locale_index;
            return BUILTIN_LOCALE_NAMES[locale_index];
        case LC_TIME:
            global_locale.time = locale_index;
            return BUILTIN_LOCALE_NAMES[locale_index];
        case LC_ALL:
            global_locale.collate = locale_index;
            global_locale.ctype = locale_index;
            global_locale.messages = locale_index;
            global_locale.monetary = locale_index;
            global_locale.numeric = locale_index;
            global_locale.time = locale_index;
            return BUILTIN_LOCALE_NAMES[locale_index];

        default:
            return NULL;
    }
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/uselocale.html */
locale_t uselocale(locale_t newloc) {
    locale_t old = current_locale;
    if (newloc) current_locale = (struct locale*)newloc;
    return old;
}


/* Functions defined in ctype.h */
#define DEFINE_CTYPE_IS(class) \
int is##class##_l(int c, locale_t locale) { \
    const struct ctype* ctype = BUILTIN_LOCALE_CTYPES + ((struct locale*)locale)->ctype; \
    for (size_t i = 0; i < ctype->class##_len; ++i) { \
        if ((unsigned char)ctype->class[i] == c) return 1; \
    } \
    return 0; \
}

#define DEFINE_CTYPE_TO(class) \
int to##class##_l(int c, locale_t locale) { \
    const struct ctype* ctype = BUILTIN_LOCALE_CTYPES + ((struct locale*)locale)->ctype; \
    for (size_t i = 0; i < ctype->to##class##_len; ++i) { \
        if ((unsigned char)ctype->to##class##_from[i] == c) return (unsigned char)ctype->to##class##_to[i]; \
    } \
    return c; \
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isalnum_l.html */
int isalnum_l(int c, locale_t locale) {
    return isalpha_l(c, locale) || isdigit_l(c, locale);
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isalpha_l.html */
DEFINE_CTYPE_IS(alpha)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isblank_l.html */
DEFINE_CTYPE_IS(blank)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/iscntrl_l.html */
DEFINE_CTYPE_IS(cntrl)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isdigit_l.html */
DEFINE_CTYPE_IS(digit)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isgraph_l.html */
DEFINE_CTYPE_IS(graph)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/islower_l.html */
DEFINE_CTYPE_IS(lower)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isprint_l.html */
DEFINE_CTYPE_IS(print)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/ispunct_l.html */
DEFINE_CTYPE_IS(punct)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isspace_l.html */
DEFINE_CTYPE_IS(space)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isupper_l.html */
DEFINE_CTYPE_IS(upper)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/isxdigit_l.html */
DEFINE_CTYPE_IS(xdigit)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/tolower_l.html */
DEFINE_CTYPE_TO(lower)

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/toupper_l.html */
DEFINE_CTYPE_TO(upper)
