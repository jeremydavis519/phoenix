# Copyright (c) 2021-2022 Jeremy Davis (jeremydavis519@gmail.com)
#
# Permission is hereby granted, free of charge, to any person obtaining a copy of this software
# and associated documentation files (the "Software"), to deal in the Software without restriction,
# including without limitation the rights to use, copy, modify, merge, publish, distribute,
# sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all copies or
# substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
# NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
# NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
# DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

.PHONY: all aarch64-phoenix test clean

.DELETE_ON_ERROR:

DEBUG_OR_RELEASE ?= release

SRC = $(wildcard src/*.c)
HEADERS = $(wildcard include/*.h)
OBJ = $(patsubst src/%.c, build/%.o, $(SRC))

ifeq ($(DEBUG_OR_RELEASE),debug)
	CC_FLAGS := -Og
	CARGO_FLAGS :=
else
	CC_FLAGS := -O3
	CARGO_FLAGS := --release
endif

all: lib/libc.a lib/libg.a

aarch64-phoenix:
	CC=aarch64-none-elf-gcc \
	AR=aarch64-none-elf-ar \
	TARGET=aarch64-phoenix \
	$(MAKE)

test: aarch64-phoenix

lib/libc.a: $(OBJ)
	mkdir -p lib
	rm -f lib/libc.a
	$(AR) rcu lib/libc.a $^

lib/libg.a: lib/libc.a
	cp $< $@

clean:
	rm -rf build lib

build/%.o: src/%.c ../libphoenix/target/$(TARGET)/$(DEBUG_OR_RELEASE)/liblibphoenix.a
	mkdir -p build
	$(CC) $(CC_FLAGS) -ffreestanding -Wall -Wextra -Wpedantic -c \
		-Iinclude -L../libphoenix/target/$(TARGET)/$(DEBUG_OR_RELEASE) -llibphoenix \
		-o $@ $<

src/%.c: $(HEADERS)

../libphoenix/target/$(TARGET)/$(DEBUG_OR_RELEASE)/liblibphoenix.a:
	cd ../libphoenix && \
	xargo build $(CARGO_FLAGS) --target=$(TARGET)
