/* Copyright (c) 2022 Jeremy Davis (jeremydavis519@gmail.com)
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

void            thread_exit(int32_t status);
void            thread_sleep(uint64_t milliseconds);
size_t          thread_spawn(void (*entry_point)(void*), void* argument, uint8_t priority, size_t stack_size);

void            process_exit(int32_t status);

size_t          device_claim(char* name, size_t len);

typedef struct {
    void*   virt;
    size_t  phys;
} VirtPhysAddr;

void            memory_free(void* ptr);
void*           memory_alloc(size_t size, size_t align);
VirtPhysAddr    memory_alloc_phys(size_t size, size_t align, size_t max_bits);
void*           memory_alloc_shared(size_t size);
size_t          memory_page_size(void);

uint64_t        time_now_unix(void);
uint64_t        time_now_unix_nanos(void);

#ifdef __cplusplus
}
#endif /* __cplusplus */

#endif /* __PHOENIX_PHOENIX_H */
