/* Copyright (c) 2022-2024 Jeremy Davis (jeremydavis519@gmail.com)
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

/* Bindings to libphoenix */

#ifndef __PHOENIX_PHOENIX_H
#define __PHOENIX_PHOENIX_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif /* __cplusplus */

#if !defined(__cplusplus) && __STDC_VERSION__ >= 199901L
#define _PHOENIX_restrict restrict
#else
#define _PHOENIX_restrict
#endif /* __cplusplus and __STDC_VERSION__ */

void                  _PHOENIX_thread_exit(int32_t status);
void                  _PHOENIX_thread_sleep(uint64_t nanoseconds);
size_t                _PHOENIX_thread_spawn(void (*entry_point)(void*), void* argument, uint8_t priority, size_t stack_size);

void                  _PHOENIX_process_exit(int32_t status);

size_t                _PHOENIX_device_claim(char* name, size_t len);

typedef struct {
    void*   virt;
    size_t  phys;
} _PHOENIX_VirtPhysAddr;

void                  _PHOENIX_memory_free(void* ptr);
void*                 _PHOENIX_memory_alloc(size_t size, size_t align);
_PHOENIX_VirtPhysAddr _PHOENIX_memory_alloc_phys(size_t size, size_t align, size_t max_bits);
void*                 _PHOENIX_memory_alloc_shared(size_t size);
size_t                _PHOENIX_memory_page_size(void);

uint64_t              _PHOENIX_time_now_unix(void);
uint64_t              _PHOENIX_time_now_unix_nanos(void);

typedef struct _PHOENIX_PipeReader _PHOENIX_PipeReader;
typedef struct _PHOENIX_PipeWriter _PHOENIX_PipeWriter;
int8_t                _PHOENIX_pipe_new(_PHOENIX_PipeReader** _PHOENIX_restrict reader, _PHOENIX_PipeWriter** _PHOENIX_restrict writer);
void                  _PHOENIX_pipe_free_reader(_PHOENIX_PipeReader* reader);
void                  _PHOENIX_pipe_free_writer(_PHOENIX_PipeWriter* writer);
ssize_t               _PHOENIX_pipe_read(_PHOENIX_PipeReader* _PHOENIX_restrict reader, char* _PHOENIX_restrict buf, ssize_t count);
ssize_t               _PHOENIX_pipe_write(_PHOENIX_PipeWriter* _PHOENIX_restrict writer, const char* _PHOENIX_restrict buf, ssize_t count);

#ifdef __cplusplus
}
#endif /* __cplusplus */

#endif /* __PHOENIX_PHOENIX_H */
