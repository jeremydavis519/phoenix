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

/* This file defines the C standard library's I/O functions and types for applications written for
 * Phoenix. Since everything in here is standard, see http://www.cplusplus.com/reference/cstdio/
 * for docs. */

#ifndef _STDINT_H
#define _STDINT_H

#include <limits.h>

/*
__INTPTR_MAX__
__UINTPTR_MAX__
__INTMAX_MAX__
__UINTMAX_MAX__
__PTRDIFF_MAX__
__SIG_ATOMIC_MAX__
__SIG_ATOMIC_MIN__
__SIZE_MAX__
__WCHAR_MAX__
__WCHAR_MIN__
__WINT_MAX__
__WINT_MIN__
__INT8_C(c)
__INT16_C(c)
__INT32_C(c)
__INT64_C(c)
__UINT8_C(c)
__UINT16_C(c)
__UINT32_C(c)
__UINT64_C(c)
__INTMAX_C(c)
__UINTMAX_C(c)*/

typedef long long int intmax_t;
typedef unsigned long long int uintmax_t;
#define INTMAX_MIN LLONG_MIN
#define INTMAX_MAX LLONG_MAX
#define INTMAX_C(c) (c##ll)
#define UINTMAX_MAX ULLONG_MAX
#define UINTMAX_C(c) (c##ull)

#if UCHAR_MAX == 0xff
typedef signed char int8_t;
typedef unsigned char uint8_t;
#define int8_t int8_t
#define INT8_MIN SCHAR_MIN
#define INT8_MAX SCHAR_MAX
#define INT8_C(c) (int8_t)(c)
#define UINT8_MAX UCHAR_MAX
#define UINT8_C(c) (uint8_t)(c##u)
#elif USHRT_MAX == 0xff
typedef short int int8_t;
typedef unsigned short int uint8_t;
#define int8_t int8_t
#define INT8_MIN SHRT_MIN
#define INT8_MAX SHRT_MAX
#define INT8_C(c) (int8_t)(c)
#define UINT8_MAX SHRT_MAX
#define UINT8_C(c) (uint8_t)(c##u)
#elif UINT_MAX == 0xff
typedef int int8_t;
typedef unsigned int uint8_t;
#define int8_t int8_t
#define INT8_MIN INT_MIN
#define INT8_MAX INT_MAX
#define INT8_C(c) (c)
#define UINT8_MAX UINT_MAX
#define UINT8_C(c) (c##u)
#elif ULONG_MAX == 0xff
typedef long int int8_t;
typedef unsigned long int uint8_t;
#define int8_t int8_t
#define INT8_MIN LONG_MIN
#define INT8_MAX LONG_MAX
#define INT8_C(c) (c##l)
#define UINT8_MAX ULONG_MAX
#define UINT8_C(c) (c##ul)
#elif ULLONG_MAX == 0xff
typedef long long int int8_t;
typedef unsigned long long int uint8_t;
#define int8_t int8_t
#define INT8_MIN LLONG_MIN
#define INT8_MAX LLONG_MAX
#define INT8_C(c) (c##ll)
#define UINT8_MAX ULLONG_MAX
#define UINT8_C(c) (c##ull)
#endif

#if UCHAR_MAX == 0xffff
typedef signed char int16_t;
typedef unsigned char uint16_t;
#define int16_t int16_t
#define INT16_MIN SCHAR_MIN
#define INT16_MAX SCHAR_MAX
#define INT16_C(c) (int16_t)(c)
#define UINT16_MAX UCHAR_MAX
#define UINT16_C(c) (uint16_t)(c##u)
#elif USHRT_MAX == 0xffff
typedef short int int16_t;
typedef unsigned short int uint16_t;
#define int16_t int16_t
#define INT16_MIN SHRT_MIN
#define INT16_MAX SHRT_MAX
#define INT16_C(c) (int16_t)(c)
#define UINT16_MAX SHRT_MAX
#define UINT16_C(c) (uint16_t)(c##u)
#elif UINT_MAX == 0xffff
typedef int int16_t;
typedef unsigned int uint16_t;
#define int16_t int16_t
#define INT16_MIN INT_MIN
#define INT16_MAX INT_MAX
#define INT16_C(c) (c)
#define UINT16_MAX UINT_MAX
#define UINT16_C(c) (c##u)
#elif ULONG_MAX == 0xffff
typedef long int int16_t;
typedef unsigned long int uint16_t;
#define int16_t int16_t
#define INT16_MIN LONG_MIN
#define INT16_MAX LONG_MAX
#define INT16_C(c) (c##l)
#define UINT16_MAX ULONG_MAX
#define UINT16_C(c) (c##ul)
#elif ULLONG_MAX == 0xffff
typedef long long int int16_t;
typedef unsigned long long int uint16_t;
#define int16_t int16_t
#define INT16_MIN LLONG_MIN
#define INT16_MAX LLONG_MAX
#define INT16_C(c) (c##ll)
#define UINT16_MAX ULLONG_MAX
#define UINT16_C(c) (c##ull)
#endif

#if UCHAR_MAX == 0xffffffff
typedef signed char int32_t;
typedef unsigned char uint32_t;
#define int32_t int32_t
#define INT32_MIN SCHAR_MIN
#define INT32_MAX SCHAR_MAX
#define INT32_C(c) (int32_t)(c)
#define UINT32_MAX UCHAR_MAX
#define UINT32_C(c) (uint32_t)(c##u)
#elif USHRT_MAX == 0xffffffff
typedef short int int32_t;
typedef unsigned short int uint32_t;
#define int32_t int32_t
#define INT32_MIN SHRT_MIN
#define INT32_MAX SHRT_MAX
#define INT32_C(c) (int32_t)(c)
#define UINT32_MAX SHRT_MAX
#define UINT32_C(c) (uint32_t)(c##u)
#elif UINT_MAX == 0xffffffff
typedef int int32_t;
typedef unsigned int uint32_t;
#define int32_t int32_t
#define INT32_MIN INT_MIN
#define INT32_MAX INT_MAX
#define INT32_C(c) (c)
#define UINT32_MAX UINT_MAX
#define UINT32_C(c) (c##u)
#elif ULONG_MAX == 0xffffffff
typedef long int int32_t;
typedef unsigned long int uint32_t;
#define int32_t int32_t
#define INT32_MIN LONG_MIN
#define INT32_MAX LONG_MAX
#define INT32_C(c) (c##l)
#define UINT32_MAX ULONG_MAX
#define UINT32_C(c) (c##ul)
#elif ULLONG_MAX == 0xffffffff
typedef long long int int32_t;
typedef unsigned long long int uint32_t;
#define int32_t int32_t
#define INT32_MIN LLONG_MIN
#define INT32_MAX LLONG_MAX
#define INT32_C(c) (c##ll)
#define UINT32_MAX ULLONG_MAX
#define UINT32_C(c) (c##ull)
#endif

#if UCHAR_MAX == 0xffffffffffffffff
typedef signed char int64_t;
typedef unsigned char uint64_t;
#define int64_t int64_t
#define INT64_MIN SCHAR_MIN
#define INT64_MAX SCHAR_MAX
#define INT64_C(c) (int64_t)(c)
#define UINT64_MAX UCHAR_MAX
#define UINT64_C(c) (uint64_t)(c##u)
#elif USHRT_MAX == 0xffffffffffffffff
typedef short int int64_t;
typedef unsigned short int uint64_t;
#define int64_t int64_t
#define INT64_MIN SHRT_MIN
#define INT64_MAX SHRT_MAX
#define INT64_C(c) (int64_t)(c)
#define UINT64_MAX SHRT_MAX
#define UINT64_C(c) (uint64_t)(c##u)
#elif UINT_MAX == 0xffffffffffffffff
typedef int int64_t;
typedef unsigned int uint64_t;
#define int64_t int64_t
#define INT64_MIN INT_MIN
#define INT64_MAX INT_MAX
#define INT64_C(c) (c)
#define UINT64_MAX UINT_MAX
#define UINT64_C(c) (c##u)
#elif ULONG_MAX == 0xffffffffffffffff
typedef long int int64_t;
typedef unsigned long int uint64_t;
#define int64_t int64_t
#define INT64_MIN LONG_MIN
#define INT64_MAX LONG_MAX
#define INT64_C(c) (c##l)
#define UINT64_MAX ULONG_MAX
#define UINT64_C(c) (c##ul)
#elif ULLONG_MAX == 0xffffffffffffffff
typedef long long int int64_t;
typedef unsigned long long int uint64_t;
#define int64_t int64_t
#define INT64_MIN LLONG_MIN
#define INT64_MAX LLONG_MAX
#define INT64_C(c) (c##ll)
#define UINT64_MAX ULLONG_MAX
#define UINT64_C(c) (c##ull)
#endif

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
#define SIG_ATOMIC_MIN __SIG_ATOMIC_MIN__
#define SIG_ATOMIC_MAX __SIG_ATOMIC_MAX__
#define WCHAR_MIN __WCHAR_MIN__
#define WCHAR_MAX __WCHAR_MAX__
#define WINT_MIN WCHAR_MIN
#define WINT_MAX WCHAR_MAX

#endif /* _STDINT_H */
