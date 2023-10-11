/* Copyright (c) 2022-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

#ifndef __PHOENIX_ERRNO_H
#define __PHOENIX_ERRNO_H

#ifdef __cplusplus
extern "C" {
#endif

extern __thread int errno;

#ifdef __cplusplus
}
#endif

#define E2BIG            1
#define EACCES           2
#define EADDRINUSE       3
#define EADDRNOTAVAIL    4
#define EAFNOSUPPORT     5
#define EAGAIN           6
#define EALREADY         7
#define EBADF            8
#define EBADMSG          9
#define EBUSY           10
#define ECANCELED       11
#define ECHILD          12
#define ECONNABORTED    13
#define ECONNREFUSED    14
#define ECONNRESET      15
#define EDEADLK         16
#define EDESTADDRREQ    17
#define EDOM            18
#define EDQUOT          19
#define EEXIST          20
#define EFAULT          21
#define EFBIG           22
#define EHOSTUNREACH    23
#define EIDRM           24
#define EILSEQ          25
#define EINPROGRESS     26
#define EINTR           27
#define EINVAL          28
#define EIO             29
#define EISCONN         30
#define EISDIR          31
#define ELOOP           32
#define EMFILE          33
#define EMLINK          34
#define EMSGSIZE        35
#define EMULTIHOP       36
#define ENAMETOOLONG    37
#define ENETDOWN        38
#define ENETRESET       39
#define ENETUNREACH     40
#define ENFILE          41
#define ENOBUFS         42
#define ENODATA         43
#define ENODEV          44
#define ENOENT          45
#define ENOEXEC         46
#define ENOLCK          47
#define ENOLINK         48
#define ENOMEM          49
#define ENOMSG          50
#define ENOPROTOOPT     51
#define ENOSPC          52
#define ENOSR           53
#define ENOSTR          54
#define ENOSYS          55
#define ENOTCONN        56
#define ENOTDIR         57
#define ENOTEMPTY       58
#define ENOTRECOVERABLE 59
#define ENOTSOCK        60
#define ENOTSUP         61
#define ENOTTY          62
#define ENXIO           63
#define EOPNOTSUPP      64
#define EOVERFLOW       65
#define EOWNERDEAD      66
#define EPERM           67
#define EPIPE           68
#define EPROTO          69
#define EPROTONOSUPPORT 70
#define EPROTOTYPE      71
#define ERANGE          72
#define EROFS           73
#define ESPIPE          74
#define ESRCH           75
#define ESTALE          76
#define ETIME           77
#define ETIMEDOUT       78
#define ETXTBSY         79
#define EWOULDBLOCK     80
#define EXDEV           81

#define EINTERNAL       -1 /* Indicates an internal error in a system library. Ideally, client code should never see this. */

#endif /* __PHOENIX_ERRNO_H */
