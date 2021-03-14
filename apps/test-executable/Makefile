# Copyright (c) 2020-2021 Jeremy Davis (jeremydavis519@gmail.com)
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
AS_CPU ?= cortex-a53
ifeq ($(AS_CPU),cortex-a35)
	AS_SYMS ?= -defsym _armv8_=1
else ifeq ($(AS_CPU),cortex-a53)
	AS_SYMS ?= -defsym _armv8_=1
else ifeq ($(AS_CPU),cortex-a55)
	AS_SYMS ?= -defsym _armv8_=1 -defsym _armv8_1_=1 -defsym _armv8_2_=1
else ifeq ($(AS_CPU),cortex-a57)
	AS_SYMS ?= -defsym _armv8_=1
else ifeq ($(AS_CPU),cortex-a72)
	AS_SYMS ?= -defsym _armv8_=1
else ifeq ($(AS_CPU),cortex-a73)
	AS_SYMS ?= -defsym _armv8_=1
else ifeq ($(AS_CPU),cortex-a75)
	AS_SYMS ?= -defsym _armv8_=1 -defsym _armv8_1_=1 -defsym _armv8_2_=1
else ifeq ($(AS_CPU),cortex-a76)
	AS_SYMS ?= -defsym _armv8_=1 -defsym _armv8_1_=1 -defsym _armv8_2_=1
else ifeq ($(AS_CPU),exynos-m1)
	AS_SYMS ?= -defsym _armv8_=1 -defsym _armv8_1_=1 -defsym _armv8_2_=1
else ifeq ($(AS_CPU),falkor)
	# TODO: Find out which CPU this is.
else ifeq ($(AS_CPU),qdf24xx)
	# TODO: Find out which CPU this is.
else ifeq ($(AS_CPU),saphira)
	# TODO: Find out which CPU this is.
else ifeq ($(AS_CPU),thunderx)
	AS_SYMS ?= -defsym _armv8_=1 -defsym _armv8_1_=1 -defsym _armv8_2_=1
else ifeq ($(AS_CPU),vulcan)
	# TODO: Find out which CPU this is.
else ifeq ($(AS_CPU),xgene1)
	AS_SYMS ?= -defsym _armv8_=1 -defsym _armv8_1_=1 -defsym _armv8_2_=1
else ifeq ($(AS_CPU),xgene2)
	AS_SYMS ?= -defsym _armv8_=1 -defsym _armv8_1_=1 -defsym _armv8_2_=1
else ifeq ($(AS_CPU),all)
	AS_SYMS ?= -defsym _armv8_=1 -defsym _armv8_1_=1 -defsym _armv8_2_=1
endif
TARGET ?= $(ARCH)-phoenix
TARGET_TRIPLE ?= $(ARCH)-none-elf
ASSEMBLER ?= $(TARGET_TRIPLE)-as
AS_PARAMS ?=
CARGO_FEATURES ?=
LINKER ?= $(TARGET_TRIPLE)-ld
LD_PARAMS ?= -n --gc-sections

PWD := $(shell pwd)
CYGPATH := $(shell which cygpath)
ifdef CYGPATH
	PWD := $(shell $(CYGPATH) -m $(PWD))
endif

# A recursive wildcard function for finding all files that match a pattern in the given directory and all subdirectories
rwildcard = $(wildcard $1$2) $(foreach d,$(wildcard $1*),$(call rwildcard,$d/,$2))

ASM_SRC := $(call rwildcard,src/,*.S)
RUST_SRC := $(call rwildcard,./,*.rs)

ARCH_OBJ = $(patsubst src/%.S, build/$(DEBUG_OR_RELEASE)/%.o, $(ASM_SRC))
MULTIARCH_LIB = target/$(TARGET)/$(DEBUG_OR_RELEASE)/libtest_executable.a

BIN = build/$(DEBUG_OR_RELEASE)/test.exe

DEBUG_OR_RELEASE ?= debug

ifeq ($(DEBUG_OR_RELEASE),debug)
	CARGO_RELEASE :=
	AS_DEBUG := -g
	LD_RELEASE :=
else
	CARGO_RELEASE := --release
	AS_DEBUG :=
	LD_RELEASE := -O -s
endif

.PHONY: all clean release

all: $(BIN)

clean:
	rm -rf build target && xargo clean

release:
	@$(MAKE) --eval="DEBUG_OR_RELEASE := release" all

$(BIN): linker.ld $(ARCH_OBJ) $(MULTIARCH_LIB)
	@mkdir -p "$(shell dirname $@)"
	$(LINKER) $(LD_PARAMS) $(LD_RELEASE) -o "$(BIN)" -T $^

target/$(TARGET)/$(DEBUG_OR_RELEASE)/%.o: src/arch/$(ARCH)/%.S
	@mkdir -p "$(shell dirname $@)"
	$(ASSEMBLER) -I$(shell dirname $<) $(AS_PARAMS) -mcpu="$(AS_CPU)" $(AS_SYMS) $(AS_DEBUG) $< -o $@

$(MULTIARCH_LIB): $(RUST_SRC) $(TARGET).json Cargo.toml
	RUST_TARGET_PATH="$(PWD)/../.." \
	xargo build $(CARGO_PARAMS) $(CARGO_RELEASE) --target "$(TARGET)"
