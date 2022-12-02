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

#include <errno.h>
#include <stddef.h>
#include <string.h>

/* Copying */
void* memcpy(void* restrict dest, const void* restrict src, size_t count) {
    unsigned char* sdest = dest;
    const unsigned char* ssrc = src;
    while (count--) {
        *sdest++ = *ssrc++;
    }
    return dest;
}

void* memmove(void* dest, const void* src, size_t count) {
    unsigned char* sdest = dest;
    const unsigned char* ssrc = src;
    if (dest < src) {
        while (count--) {
            *sdest++ = *ssrc++;
        }
    } else {
        /* Avoid overwriting not-yet-used src bytes by copying backwards. */
        sdest += count;
        ssrc += count;
        while (count--) {
            *--sdest = *--ssrc;
        }
    }
    return dest;
}

char* strcpy(char* restrict dest, const char* restrict src) {
    char c;
    while ((c = *src++)) {
        *dest++ = c;
    }
    *dest = '\0';
    return dest;
}

char* strncpy(char* restrict dest, const char* restrict src, size_t count) {
    char* sdest = dest;
    while (count--) {
        if (!(*sdest++ = *src++)) {
            break;
        }
    }

    /* The rest of the array needs to be padded with null characters. */
    memset(sdest, '\0', count);

    return dest;
}


/* Concatenation */
/* TODO
char* strcat(char* dest, const char* src);
char* strncat(char* dest, const char* src, size_t count); */


/* Comparison */
/* TODO
int memcmp(const void* ptr1, const void* ptr2, size_t count); */

int strcmp(const char* s1, const char* s2) {
    while (*s1 && *s2) {
        int result = (int)*s1++ - (int)*s2++;
        if (result) {
            return result;
        }
    }
    return (int)*s1 - (int)*s2;
}

/* TODO
int strcoll(const char* s1, const char* s2);
int strncmp(const char* s1, const char* s2, size_t count);
int strxfrm(char* restrict dest, const char* restrict src, size_t count); */


/* Searching */
/* TODO
void* memchr(const void* ptr, int value, size_t count);
char* strchr(const char* s, int c);
size_t strcspn(const char* s1, const char* s2);
char* strpbrk(const char* s1, const char* s2);
char* strrchr(const char* s, int c);
size_t strspn(const char* s1, const char* s2);
char* strstr(const char* s1, const char* s2);
char* strtok(char* restrict s, const char* restrict delimiters); */


/* Other */
void* memset(void* dest, int ch, size_t count) {
    unsigned char* sdest = dest;
    while (count--) {
        *sdest++ = ch;
    }
    return dest;
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/strerror.html */
char* strerror(int errnum) {
    return strerror_l(errnum, uselocale((locale_t)0));
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/strerror.html */
char* strerror_l(int errnum, locale_t locale) {
    /* FIXME: Actually use the locale. */
    switch (errnum) {
    case 0:                 return "No error";
    case E2BIG:             return "Argument list too long";
    case EACCES:            return "Permission denied";
    case EADDRINUSE:        return "Address in use";
    case EADDRNOTAVAIL:     return "Address not available";
    case EAFNOSUPPORT:      return "Address family not supported";
    case EAGAIN:            return "Resource unavailable, try again";
    case EALREADY:          return "Connection already in progress";
    case EBADF:             return "Bad file descriptor";
    case EBADMSG:           return "Bad message";
    case EBUSY:             return "Device or resource busy";
    case ECANCELED:         return "Operation canceled";
    case ECHILD:            return "No child processes";
    case ECONNABORTED:      return "Connection aborted";
    case ECONNREFUSED:      return "Connection refused";
    case ECONNRESET:        return "Connection reset";
    case EDEADLK:           return "Resource deadlock would occur";
    case EDESTADDRREQ:      return "Destination address required";
    case EDOM:              return "Mathematics argument out of domain of function";
    case EDQUOT:            return "EDQUOT (reserved errno value)";
    case EEXIST:            return "File exists";
    case EFAULT:            return "Bad address";
    case EFBIG:             return "File too large";
    case EHOSTUNREACH:      return "Host is unreachable";
    case EIDRM:             return "Identifier removed";
    case EILSEQ:            return "Illegal byte sequence";
    case EINPROGRESS:       return "Operation in progress";
    case EINTR:             return "Interrupted function";
    case EINVAL:            return "Invalid argument";
    case EIO:               return "I/O error";
    case EISCONN:           return "Socket is connected";
    case EISDIR:            return "Is a directory";
    case ELOOP:             return "Too many levels of symbolic links";
    case EMFILE:            return "File descriptor value too large";
    case EMLINK:            return "Too many links";
    case EMSGSIZE:          return "Message too large";
    case EMULTIHOP:         return "EMULTIHOP (reserved errno value)";
    case ENAMETOOLONG:      return "Filename too long";
    case ENETDOWN:          return "Network is down";
    case ENETRESET:         return "Connection aborted by network";
    case ENETUNREACH:       return "Network unreachable";
    case ENFILE:            return "Too many files open in system";
    case ENOBUFS:           return "No buffer space available";
    case ENODATA:           return "No message available on the STREAM head read queue";
    case ENODEV:            return "No such device";
    case ENOENT:            return "No such file or directory";
    case ENOEXEC:           return "Executable file format error";
    case ENOLCK:            return "No locks available";
    case ENOLINK:           return "ENOLINK (reserved errno value)";
    case ENOMEM:            return "Not enough space";
    case ENOMSG:            return "No message of the desired type";
    case ENOPROTOOPT:       return "Protocol not available";
    case ENOSPC:            return "No space left on device";
    case ENOSR:             return "No STREAM resources";
    case ENOSTR:            return "Not a STREAM";
    case ENOSYS:            return "Functionality not supported";
    case ENOTCONN:          return "The socket is not connected";
    case ENOTDIR:           return "Not a directory or a symbolic link to a directory";
    case ENOTEMPTY:         return "Directory not empty";
    case ENOTRECOVERABLE:   return "State not recoverable";
    case ENOTSOCK:          return "Not a socket";
    case ENOTSUP:           return "Not supported";
    case ENOTTY:            return "Inappropriate I/O control operation";
    case ENXIO:             return "No such device or address";
    case EOPNOTSUPP:        return "Operation not supported on socket";
    case EOVERFLOW:         return "Value too large to be stored in data type";
    case EOWNERDEAD:        return "Previous owner died";
    case EPERM:             return "Operation not permitted";
    case EPIPE:             return "Broken pipe";
    case EPROTO:            return "Protocol error";
    case EPROTONOSUPPORT:   return "Protocol not supported";
    case EPROTOTYPE:        return "Protocol wrong type for socket";
    case ERANGE:            return "Result too large";
    case EROFS:             return "Read-only file system";
    case ESPIPE:            return "Invalid seek";
    case ESRCH:             return "No such process";
    case ESTALE:            return "ESTALE (reserved errno value)";
    case ETIME:             return "Stream ioctl() timeout";
    case ETIMEDOUT:         return "Connection timed out";
    case ETXTBSY:           return "Text file busy";
    case EWOULDBLOCK:       return "Operation would block";
    case EXDEV:             return "Cross-device link";
    default:
        errno = EINVAL;
        return "Unknown error";
    }
}

/* https://pubs.opengroup.org/onlinepubs/9699919799/functions/strerror.html */
int strerror_r(int errnum, char* strerrbuf, size_t buflen) {
    /* Get error message */
    const char* s = strerror(errnum);

    /* Copy to buffer */
    while (buflen--) {
        if (!(*strerrbuf++ = *s++)) return 0;
    }

    /* Ran out of room in the buffer */
    return ERANGE;
}

size_t strlen(const char* s) {
    size_t len = 0;
    while (*s++) {
        ++len;
    }
    return len;
}
