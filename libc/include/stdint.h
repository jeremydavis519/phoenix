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

/* Integer types defined by the ISO C standard and extended by POSIX
   https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/stdint.h.html */

#ifndef _STDINT_H
#define _STDINT_H

#include <limits.h>

typedef __INTMAX_TYPE__ intmax_t;
typedef __UINTMAX_TYPE__ uintmax_t;
#define INTMAX_MIN __INTMAX_MIN__
#define INTMAX_MAX __INTMAX_MAX__
#define INTMAX_C __INTMAX_C
#define UINTMAX_MAX __UINTMAX_MAX
#define UINTMAX_C __UINTMAX_C

#if UCHAR_MAX == 0xff
typedef signed char int8_t;
typedef unsigned char uint8_t;
#define int8_t int8_t
#define INT8_MIN SCHAR_MIN
#define INT8_MAX SCHAR_MAX
#define UINT8_MAX UCHAR_MAX
#elif USHRT_MAX == 0xff
typedef short int int8_t;
typedef unsigned short int uint8_t;
#define int8_t int8_t
#define INT8_MIN SHRT_MIN
#define INT8_MAX SHRT_MAX
#define UINT8_MAX SHRT_MAX
#elif UINT_MAX == 0xff
typedef int int8_t;
typedef unsigned int uint8_t;
#define int8_t int8_t
#define INT8_MIN INT_MIN
#define INT8_MAX INT_MAX
#define UINT8_MAX UINT_MAX
#elif ULONG_MAX == 0xff
typedef long int int8_t;
typedef unsigned long int uint8_t;
#define int8_t int8_t
#define INT8_MIN LONG_MIN
#define INT8_MAX LONG_MAX
#define UINT8_MAX ULONG_MAX
#elif ULLONG_MAX == 0xff
typedef long long int int8_t;
typedef unsigned long long int uint8_t;
#define int8_t int8_t
#define INT8_MIN LLONG_MIN
#define INT8_MAX LLONG_MAX
#define UINT8_MAX ULLONG_MAX
#endif
#define INT8_C __INT8_C
#define UINT8_C __UINT8_C

#if UCHAR_MAX == 0xffff
typedef signed char int16_t;
typedef unsigned char uint16_t;
#define int16_t int16_t
#define INT16_MIN SCHAR_MIN
#define INT16_MAX SCHAR_MAX
#define UINT16_MAX UCHAR_MAX
#elif USHRT_MAX == 0xffff
typedef short int int16_t;
typedef unsigned short int uint16_t;
#define int16_t int16_t
#define INT16_MIN SHRT_MIN
#define INT16_MAX SHRT_MAX
#define UINT16_MAX SHRT_MAX
#elif UINT_MAX == 0xffff
typedef int int16_t;
typedef unsigned int uint16_t;
#define int16_t int16_t
#define INT16_MIN INT_MIN
#define INT16_MAX INT_MAX
#define UINT16_MAX UINT_MAX
#elif ULONG_MAX == 0xffff
typedef long int int16_t;
typedef unsigned long int uint16_t;
#define int16_t int16_t
#define INT16_MIN LONG_MIN
#define INT16_MAX LONG_MAX
#define UINT16_MAX ULONG_MAX
#elif ULLONG_MAX == 0xffff
typedef long long int int16_t;
typedef unsigned long long int uint16_t;
#define int16_t int16_t
#define INT16_MIN LLONG_MIN
#define INT16_MAX LLONG_MAX
#define UINT16_MAX ULLONG_MAX
#endif
#define INT16_C __INT16_C
#define UINT16_C __UINT16_C

#if UCHAR_MAX == 0xffffffff
typedef signed char int32_t;
typedef unsigned char uint32_t;
#define int32_t int32_t
#define INT32_MIN SCHAR_MIN
#define INT32_MAX SCHAR_MAX
#define UINT32_MAX UCHAR_MAX
#elif USHRT_MAX == 0xffffffff
typedef short int int32_t;
typedef unsigned short int uint32_t;
#define int32_t int32_t
#define INT32_MIN SHRT_MIN
#define INT32_MAX SHRT_MAX
#define UINT32_MAX SHRT_MAX
#elif UINT_MAX == 0xffffffff
typedef int int32_t;
typedef unsigned int uint32_t;
#define int32_t int32_t
#define INT32_MIN INT_MIN
#define INT32_MAX INT_MAX
#define UINT32_MAX UINT_MAX
#elif ULONG_MAX == 0xffffffff
typedef long int int32_t;
typedef unsigned long int uint32_t;
#define int32_t int32_t
#define INT32_MIN LONG_MIN
#define INT32_MAX LONG_MAX
#define UINT32_MAX ULONG_MAX
#elif ULLONG_MAX == 0xffffffff
typedef long long int int32_t;
typedef unsigned long long int uint32_t;
#define int32_t int32_t
#define INT32_MIN LLONG_MIN
#define INT32_MAX LLONG_MAX
#define UINT32_MAX ULLONG_MAX
#endif
#define INT32_C __INT32_C
#define UINT32_C __UINT32_C

#if UCHAR_MAX == 0xffffffffffffffff
typedef signed char int64_t;
typedef unsigned char uint64_t;
#define int64_t int64_t
#define INT64_MIN SCHAR_MIN
#define INT64_MAX SCHAR_MAX
#define UINT64_MAX UCHAR_MAX
#elif USHRT_MAX == 0xffffffffffffffff
typedef short int int64_t;
typedef unsigned short int uint64_t;
#define int64_t int64_t
#define INT64_MIN SHRT_MIN
#define INT64_MAX SHRT_MAX
#define UINT64_MAX SHRT_MAX
#elif UINT_MAX == 0xffffffffffffffff
typedef int int64_t;
typedef unsigned int uint64_t;
#define int64_t int64_t
#define INT64_MIN INT_MIN
#define INT64_MAX INT_MAX
#define UINT64_MAX UINT_MAX
#elif ULONG_MAX == 0xffffffffffffffff
typedef long int int64_t;
typedef unsigned long int uint64_t;
#define int64_t int64_t
#define INT64_MIN LONG_MIN
#define INT64_MAX LONG_MAX
#define UINT64_MAX ULONG_MAX
#elif ULLONG_MAX == 0xffffffffffffffff
typedef long long int int64_t;
typedef unsigned long long int uint64_t;
#define int64_t int64_t
#define INT64_MIN LLONG_MIN
#define INT64_MAX LLONG_MAX
#define UINT64_MAX ULLONG_MAX
#endif
#define INT64_C __INT64_C
#define UINT64_C __UINT64_C

#ifdef int8_t
typedef int8_t int_least8_t;
typedef uint8_t uint_least8_t;
typedef int8_t int_fast8_t;
typedef uint8_t uint_fase8_t;
#define INT_LEAST8_MAX INT8_MAX
#define INT_LEAST8_MIN INT8_MIN
#define UINT_LEAST8_MAX UINT8_MAX
#define INT_FAST8_MAX INT8_MAX
#define INT_FAST8_MIN INT8_MIN
#define UINT_FAST8_MAX UINT8_MAX
#elif defined(int16_t)
typedef int16_t int_least8_t;
typedef uint16_t uint_least8_t;
typedef int16_t int_fast8_t;
typedef uint16_t uint_fast8_t;
#define INT_LEAST8_MAX INT16_MAX
#define INT_LEAST8_MIN INT16_MIN
#define UINT_LEAST8_MAX UINT16_MAX
#define INT_FAST8_MAX INT16_MAX
#define INT_FAST8_MIN INT16_MIN
#define UINT_FAST8_MAX UINT16_MAX
#elif defined(int32_t)
typedef int32_t int_least8_t;
typedef uint32_t uint_least8_t;
typedef int32_t int_fast8_t;
typedef uint32_t uint_fast8_t;
#define INT_LEAST8_MAX INT32_MAX
#define INT_LEAST8_MIN INT32_MIN
#define UINT_LEAST8_MAX UINT32_MAX
#define INT_FAST8_MAX INT32_MAX
#define INT_FAST8_MIN INT32_MIN
#define UINT_FAST8_MAX UINT32_MAX
#elif defined(int64_t)
typedef int64_t int_least8_t;
typedef uint64_t uint_least8_t;
typedef int64_t int_fast8_t;
typedef uint64_t uint_fast8_t;
#define INT_LEAST8_MAX INT64_MAX
#define INT_LEAST8_MIN INT64_MIN
#define UINT_LEAST8_MAX UINT64_MAX
#define INT_FAST8_MAX INT64_MAX
#define INT_FAST8_MIN INT64_MIN
#define UINT_FAST8_MAX UINT64_MAX
#endif

#ifdef int16_t
typedef int16_t int_least16_t;
typedef uint16_t uint_least16_t;
typedef int16_t int_fast16_t;
typedef uint16_t uint_fast16_t;
#define INT_LEAST16_MAX INT16_MAX
#define INT_LEAST16_MIN INT16_MIN
#define UINT_LEAST16_MAX UINT16_MAX
#define INT_FAST16_MAX INT16_MAX
#define INT_FAST16_MIN INT16_MIN
#define UINT_FAST16_MAX UINT16_MAX
#elif defined(int32_t)
typedef int32_t int_least16_t;
typedef uint32_t uint_least16_t;
typedef int32_t int_fast16_t;
typedef uint32_t uint_fast16_t;
#define INT_LEAST16_MAX INT32_MAX
#define INT_LEAST16_MIN INT32_MIN
#define UINT_LEAST16_MAX UINT32_MAX
#define INT_FAST16_MAX INT32_MAX
#define INT_FAST16_MIN INT32_MIN
#define UINT_FAST16_MAX UINT32_MAX
#elif defined(int64_t)
typedef int64_t int_least16_t;
typedef uint64_t uint_least16_t;
typedef int64_t int_fast16_t;
typedef uint64_t uint_fast16_t;
#define INT_LEAST16_MAX INT64_MAX
#define INT_LEAST16_MIN INT64_MIN
#define UINT_LEAST16_MAX UINT64_MAX
#define INT_FAST16_MAX INT64_MAX
#define INT_FAST16_MIN INT64_MIN
#define UINT_FAST16_MAX UINT64_MAX
#endif

#ifdef int32_t
typedef int32_t int_least32_t;
typedef uint32_t uint_least32_t;
typedef int32_t int_fast32_t;
typedef uint32_t uint_fast32_t;
#define INT_LEAST32_MAX INT32_MAX
#define INT_LEAST32_MIN INT32_MIN
#define UINT_LEAST32_MAX UINT32_MAX
#define INT_FAST32_MAX INT32_MAX
#define INT_FAST32_MIN INT32_MIN
#define UINT_FAST32_MAX UINT32_MAX
#elif defined(int64_t)
typedef int64_t int_least32_t;
typedef uint64_t uint_least32_t;
typedef int64_t int_fast32_t;
typedef uint64_t uint_fast32_t;
#define INT_LEAST32_MAX INT64_MAX
#define INT_LEAST32_MIN INT64_MIN
#define UINT_LEAST32_MAX UINT64_MAX
#define INT_FAST32_MAX INT64_MAX
#define INT_FAST32_MIN INT64_MIN
#define UINT_FAST32_MAX UINT64_MAX
#endif

#ifdef int64_t
typedef int64_t int_least64_t;
typedef uint64_t uint_least64_t;
typedef int64_t int_fast64_t;
typedef uint64_t uint_fast64_t;
#define INT_LEAST64_MAX INT64_MAX
#define INT_LEAST64_MIN INT64_MIN
#define UINT_LEAST64_MAX UINT64_MAX
#define INT_FAST64_MAX INT64_MAX
#define INT_FAST64_MIN INT64_MIN
#define UINT_FAST64_MAX UINT64_MAX
#endif

typedef __INTPTR_TYPE__ intptr_t;
typedef __UINTPTR_TYPE__ uintptr_t;
#define INTPTR_MIN __INTPTR_MIN__
#define INTPTR_MAX __INTPTR_MAX__
#define UINTPTR_MAX __UINTPTR_MAX__

#define SIZE_MAX __SIZE_MAX__
#define PTRDIFF_MIN __PTRDIFF_MIN__
#define PTRDIFF_MAX __PTRDIFF_MAX__
#define WCHAR_MIN __WCHAR_MIN__
#define WCHAR_MAX __WCHAR_MAX__
#define WINT_MIN __WINT_MIN__
#define WINT_MAX __WINT_MAX__
#define SIG_ATOMIC_MIN __SIG_ATOMIC_MIN__
#define SIG_ATOMIC_MAX __SIG_ATOMIC_MAX__

#endif /* _STDINT_H */
