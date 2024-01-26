/* Copyright (c) 2023 Jeremy Davis (jeremydavis519@gmail.com)
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

#ifndef __PHOENIX_SYS_STAT_H
#define __PHOENIX_SYS_STAT_H

#include <sys/types.h>
#include <time.h>

#ifdef __cplusplus
extern "C" {
#endif

#if !defined(__cplusplus) && __STDC_VERSION__ >= 199901L
#define _PHOENIX_restrict restrict
#else
#define _PHOENIX_restrict
#endif /* __cplusplus and __STDC_VERSION__ */

struct stat {
    dev_t           st_dev;
    ino_t           st_ino;
    mode_t          st_mode;
    nlink_t         st_nlink;
    uid_t           st_uid;
    gid_t           st_gid;
    dev_t           st_rdev;
    off_t           st_size;
    struct timespec st_atim;
    struct timespec st_mtim;
    struct timespec st_ctim;
    blksize_t       st_blksize;
    blkcnt_t        st_blocks;
};

/* For compatibility with earlier versions of POSIX */
#define st_atime st_atim.tv_sec
#define st_mtime st_mtim.tv_sec
#define st_ctime st_ctim.tv_sec

/* Values for `mode_t` file types */
#define _PHOENIX_S_IFMT   0x7000
#define S_IFMT   _PHOENIX_S_IFMT
#define _PHOENIX_S_IFBLK  0x1000
#define S_IFBLK  _PHOENIX_S_IFBLK
#define _PHOENIX_S_IFCHR  0x2000
#define S_IFCHR  _PHOENIX_S_IFCHR
#define _PHOENIX_S_IFIFO  0x3000
#define S_IFIFO  _PHOENIX_S_IFIFO
#define _PHOENIX_S_IFREG  0x4000
#define S_IFREG  _PHOENIX_S_IFREG
#define _PHOENIX_S_IFDIR  0x5000
#define S_IFDIR  _PHOENIX_S_IFDIR
#define _PHOENIX_S_IFLNK  0x6000
#define S_IFLNK  _PHOENIX_S_IFLNK
#define _PHOENIX_S_IFSOCK 0x7000
#define S_IFSOCK _PHOENIX_S_IFSOCK

#define S_ISBLK(m)  (((m) & _PHOENIX_S_IFMT) == _PHOENIX_S_IFBLK)
#define S_ISCHR(m)  (((m) & _PHOENIX_S_IFMT) == _PHOENIX_S_IFCHR)
#define S_ISFIFO(m) (((m) & _PHOENIX_S_IFMT) == _PHOENIX_S_IFIFO)
#define S_ISREG(m)  (((m) & _PHOENIX_S_IFMT) == _PHOENIX_S_IFREG)
#define S_ISDIR(m)  (((m) & _PHOENIX_S_IFMT) == _PHOENIX_S_IFDIR)
#define S_ISLNK(m)  (((m) & _PHOENIX_S_IFMT) == _PHOENIX_S_IFLNK)
#define S_ISSOCK(m) (((m) & _PHOENIX_S_IFMT) == _PHOENIX_S_IFSOCK)

#define S_TYPEISMQ(buf)  0
#define S_TYPEISSEM(buf) 0
#define S_TYPEISSHM(buf) 0
#define S_TYPEISTMO(buf) 0

/* Values for `mode_t` permissions (values prescribed by POSIX) */
#define S_ISUID 04000
#define S_ISGID 02000
#define S_ISVTX 01000
#define S_IRWXU  0700
#define S_IRUSR  0400
#define S_IWUSR  0200
#define S_IXUSR  0100
#define S_IRWXG   070
#define S_IRGRP   040
#define S_IWGRP   020
#define S_IXGRP   010
#define S_IRWXO    07
#define S_IROTH    04
#define S_IWOTH    02
#define S_IXOTH    01

#define UTIME_NOW  1000000000
#define UTIME_OMIT 1000000001

int    chmod(const char*, mode_t);
int    fchmod(int, mode_t);
int    fchmodat(int, const char*, mode_t, int);
int    fstat(int, struct stat*);
int    fstatat(int, const char* _PHOENIX_restrict, struct stat* _PHOENIX_restrict, int);
int    futimens(int, const struct timespec [2]);
int    lstat(const char* _PHOENIX_restrict, struct stat* _PHOENIX_restrict);
int    mkdir(const char*, mode_t);
int    mkdirat(int, const char*, mode_t);
int    mkfifo(const char*, mode_t);
int    mkfifoat(int, const char*, mode_t);
int    mknod(const char*, mode_t, dev_t);
int    mknodat(int, const char*, mode_t, dev_t);
int    stat(const char* _PHOENIX_restrict, struct stat* _PHOENIX_restrict);
mode_t umask(mode_t);
int    utimensat(int, const char*, const struct timespec [2], int);

#ifdef __cplusplus
}
#endif

#endif /* __PHOENIX_SYS_STAT_H */
