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

.PHONY: all aarch64-phoenix test clean null

.DELETE_ON_ERROR:

DEBUG_OR_RELEASE ?= release

SRC = $(wildcard src/*.c)
OBJ = $(patsubst src/%.c,build/%.o,$(SRC))
DEP = $(patsubst src/%.c,build/%.d,$(SRC))

LIBPHOENIX_ROOT = ../libphoenix
ifeq ($(DEBUG_OR_RELEASE),debug)
	CC_OPT := -Og
	CARGO_FLAGS :=
else
	CC_OPT := -O3
	CARGO_FLAGS := --release
endif

all: lib/libc.a lib/libg.a

# We should be able to use `-include $(DEP)` here, but for some reason make is only including
# the first file. So we concatenate them all into a single file.
-include build/dep/deps.d

aarch64-phoenix:
	$(MAKE) --eval="CC := aarch64-none-elf-gcc" --eval="AR := aarch64-none-elf-ar" --eval="TARGET := aarch64-phoenix"

test: aarch64-phoenix

lib/libc.a: $(OBJ)
	mkdir -p lib
	rm -f lib/libc.a
	$(AR) rcu lib/libc.a $(OBJ)

lib/libg.a: lib/libc.a
	cp $< $@

clean:
	rm -rf build lib

build/dep/deps.d: $(DEP) $(OBJ)
	mkdir -p build/dep
	cat $(DEP) > build/dep/deps.d

ifneq ($(TARGET),)
build/%.o: src/%.c $(LIBPHOENIX_ROOT)/clib/target/$(TARGET)/$(DEBUG_OR_RELEASE)/liblibphoenix.a Makefile
	mkdir -p build
	$(CC) $(CC_FLAGS) $(CC_OPT) -ffreestanding -Wall -Wextra -Wpedantic -c \
		-MD "-MF$(patsubst src/%.c,build/%.d,$<)" -MP \
		-Iinclude -L$(LIBPHOENIX_ROOT)/clib/target/$(TARGET)/$(DEBUG_OR_RELEASE) -llibphoenix \
		-o $@ $<

$(LIBPHOENIX_ROOT)/clib/target/$(TARGET)/$(DEBUG_OR_RELEASE)/liblibphoenix.a: null
	cd $(LIBPHOENIX_ROOT)/clib && \
	RUST_TARGET_PATH="$$PWD" \
	xargo build $(CARGO_FLAGS) --target=$(TARGET)
endif

null:
	@true
