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

/* Tests the functionality of POSIX pipes. */

#include "test.h"

#include <fcntl.h>
#include <string.h>
#include <unistd.h>

const int BUFFER_SIZE = 128;
static char reader_buffer[BUFFER_SIZE];
static char writer_buffer[BUFFER_SIZE];
static char uninitialized[BUFFER_SIZE]; /* The initial values for `reader_buffer` before each test */

static void test_blocking_read(int fildes[4]);
static void test_blocking_write(int fildes[4]);
static void test_atomic_blocking_write(int fildes[4]);
static void test_nonblocking_read(int fildes[4]);
static void test_nonblocking_write(int fildes[4]);
static void test_atomic_nonblocking_write(int fildes[4]);

int main() {
    int fildes[4] = {0};

    /* Initialize the buffers. */
    for (i = 0; i < BUFFER_SIZE; ++i) {
        /* With a buffer size of 128 or less, these produce two sequences with no elements in common
           and which are unlikely to arise by chance. */
        writer_buffer[i] = i * 7;
        uninitialized[i] = (i + 128) * 7;
    }

    errno = 0;

    /* Open two pipes (one wouldn't be enough for all the tests). */
    ASSERT(!pipe(fildes));
    ASSERT(!errno);
    ASSERT(fildes[0]);
    ASSERT(fildes[1]);

    ASSERT(!pipe(fildes + 2));
    ASSERT(!errno);
    ASSERT(fildes[2]);
    ASSERT(fildes[3]);

    /* Test blocking I/O. */
    test_blocking_read(fildes);
    test_blocking_write(fildes);
    test_atomic_blocking_write(fildes);

    /* Test non-blocking I/O. */
    fcntl(fildes[0], F_SETFL, O_NONBLOCK);
    fcntl(fildes[1], F_SETFL, O_NONBLOCK);

    test_nonblocking_read(fildes);
    test_nonblocking_write(fildes);
    test_atomic_nonblocking_write(fildes);

    errno = 0;

    /* Closing should not return an error. */
    ASSERT(!close(fildes[0]));
    ASSERT(!errno);
    ASSERT(!close(fildes[1]));
    ASSERT(!errno);

    return EXIT_SUCCESS;
}

static void test_blocking_read(int fildes[4]) {
    FIXME
}

static void test_blocking_write(int fildes[4]) {
    FIXME
}

static void test_atomic_blocking_write(int fildes[4]) {
    FIXME
}

static void test_nonblocking_read(int fildes[4]) {
    errno = 0;
    memcpy(reader_buffer, uninitialized, BUFFER_SIZE);

    /* Reading from an empty pipe with a writer should fail. */
    ASSERT(read(fildes[0], reader_buffer, 1) == -1);
    ASSERT(errno == EAGAIN);
    ASSERT(!memcmp(reader_buffer, uninitialized, BUFFER_SIZE);

    errno = 0;

    /* Writing and then reading should succeed and not overflow. */
    ASSERT(write(fildes[1], writer_buffer, 42) == 42);
    ASSERT(!errno);

    ASSERT(read(fildes[0], reader_buffer, 7) == 7);
    ASSERT(!errno);
    ASSERT(!memcmp(reader_buffer, writer_buffer, 7));
    ASSERT(!memcmp(reader_buffer + 7, uninitialized + 7, BUFFER_SIZE - 7));

    ASSERT(read(fildes[0], reader_buffer + 7, 42) == 35);
    ASSERT(!errno);
    ASSERT(!memcmp(reader_buffer, writer_buffer, 42));
    ASSERT(!memcmp(reader_buffer + 42, uninitialized + 42, BUFFER_SIZE - 42));

    /* Reading from a closed pipe should succeed with 0, indicating EOF. */
    ASSERT(!close(fildes[3]));
    ASSERT(!errno);

    ASSERT(read(fildes[2], reader_buffer, 1) == 0);
    ASSERT(!errno);
}

static void test_nonblocking_write(int fildes[4]) {
    FIXME
}

static void test_atomic_nonblocking_write(int fildes[4]) {
    FIXME
}
