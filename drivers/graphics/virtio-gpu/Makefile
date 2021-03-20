# Copyright (c) 2021 Jeremy Davis (jeremydavis519@gmail.com)
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

ARCH ?= aarch64
TARGET ?= $(ARCH)-phoenix
TARGET_TRIPLE ?= $(ARCH)-none-elf
LINKER ?= $(TARGET_TRIPLE)-ld

DEBUG_OR_RELEASE ?= debug

ifeq ($(DEBUG_OR_RELEASE),debug)
	CARGO_RELEASE :=
	RUSTFLAGS_RELEASE :=
else
	CARGO_RELEASE := --release
	RUSTFLAGS_RELEASE := -C link-arg=-O -C link-arg=-s
endif

ROOT := $(shell dirname $(shell dirname $(shell dirname $(PWD))))

.PHONY: all clean release

all:
	RUST_TARGET_PATH="$(ROOT)" \
	RUSTFLAGS="$(RUSTFLAGS) $(RUSTFLAGS_RELEASE) -C linker=$(LINKER) -C link-arg=-T -C link-arg=$(ROOT)/userspace-$(ARCH).ld" \
	xargo build $(CARGO_PARAMS) $(CARGO_RELEASE) --target "$(TARGET)"

clean:
	xargo clean

release:
	@$(MAKE) --eval="DEBUG_OR_RELEASE := release" all
