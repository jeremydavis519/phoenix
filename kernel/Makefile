# Copyright (c) 2017-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

# Supported machine types...
# ...for ARCH=aarch64:
#   qemu-virt
#   raspi3 (partial)
# ...for ARCH=arm
#   raspi2 (partial)
#   raspi1 (partial)

ARCH ?= aarch64
MACHINE ?= qemu-virt
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
MACHINE_SUBDIR ?= /qemu
TARGET ?= $(ARCH)-phoenix-eabi
TARGET_TRIPLE ?= $(ARCH)-none-elf
ASSEMBLER ?= $(TARGET_TRIPLE)-as
AS_PARAMS ?=
CARGO_FEATURES ?= all_languages
LINKER ?= $(TARGET_TRIPLE)-ld
LD_PARAMS ?= -n --gc-sections
OBJCOPY ?= $(TARGET_TRIPLE)-objcopy
OBJCOPY_PARAMS ?= -O binary
GDB ?= $(TARGET_TRIPLE)-gdb
QEMU ?= qemu-system-$(ARCH)
QEMU_PARAMS ?= -m 192M -monitor stdio -s -S
QEMU_CPU ?= cortex-a53
ifeq ($(MACHINE),raspi3)
	# Qemu doesn't recognize raspi3, but raspi2 is similar.
#	QEMU_MACHINE ?= raspi2
else ifeq ($(MACHINE),qemu-virt)
	QEMU_MACHINE ?= virt
else
	QEMU_MACHINE ?= $(MACHINE)
endif

PWD := $(shell pwd)
CYGPATH := $(shell which cygpath)
ifdef CYGPATH
	PWD := $(shell $(CYGPATH) -m $(PWD))
endif

# A recursive wildcard function for finding all files that match a pattern in the given directory and all subdirectories
rwildcard = $(wildcard $1$2) $(foreach d,$(wildcard $1*),$(call rwildcard,$d/,$2))

# Use the most specific linker file available.
LINKER_SCRIPT := $(wildcard src/arch/$(ARCH)/$(MACHINE)$(MACHINE_SUBDIR)/linker.ld)
ifeq ($(strip $(LINKER_SCRIPT)),)
	LINKER_SCRIPT := $(wildcard src/arch/$(ARCH)/$(MACHINE)/linker.ld)
	ifeq ($(strip $(LINKER_SCRIPT)),)
		LINKER_SCRIPT := src/arch/$(ARCH)/linker.ld
	endif
endif

KERNEL_ASM := $(wildcard src/arch/$(ARCH)/$(MACHINE)$(MACHINE_SUBDIR)/*.S) $(wildcard src/arch/$(ARCH)/$(MACHINE)/*.S) $(wildcard src/arch/$(ARCH)/*.S)
MULTIARCH_SRC := $(call rwildcard,./,*.rs)

ARCH_OBJ = $(patsubst src/arch/$(ARCH)/%.S, target/$(TARGET)/$(DEBUG_OR_RELEASE)/%.o, $(KERNEL_ASM)) \
	$(patsubst src/arch/$(ARCH)/%.S, target/$(TARGET)/$(DEBUG_OR_RELEASE)/%.o, $(KERNEL_ASM))
KERNEL_LIB = target/$(TARGET)/$(DEBUG_OR_RELEASE)/libphoenix_kernel.a

CARGO_TOML := $(call rwildcard,./,Cargo.toml)

KERNEL_BIN = build/$(DEBUG_OR_RELEASE)/kernel-$(ARCH).bin

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

.PHONY: all clean release test kernel

all: kernel

clean:
	rm -rf build target && xargo clean

release:
	@$(MAKE) --eval="DEBUG_OR_RELEASE := release" all

test:
	cargo test --all --features="$(CARGO_FEATURES),unit-test" $(CARGO_RELEASE) --target-dir="build/$(DEBUG_OR_RELEASE)/test" && \
	CARGO_FEATURES="$(CARGO_FEATURES),self-test" $(MAKE)

kernel: $(KERNEL_BIN)

$(KERNEL_BIN): $(LINKER_SCRIPT) $(KERNEL_LIB)
	@mkdir -p "$(shell dirname $@)"
	$(LINKER) $(LD_PARAMS) $(LD_RELEASE) -o "$(KERNEL_BIN).elf" -T $^
	$(OBJCOPY) $(OBJCOPY_PARAMS) "$(KERNEL_BIN).elf" "$(KERNEL_BIN)"

$(KERNEL_LIB): $(MULTIARCH_SRC) $(KERNEL_ASM) $(TARGET).json $(CARGO_TOML)
	RUST_TARGET_PATH="$(PWD)" \
	RUSTFLAGS="--cfg target_machine=\"$(MACHINE)\"" \
	xargo build $(CARGO_PARAMS) --features="$(CARGO_FEATURES)" $(CARGO_RELEASE) --target "$(TARGET)"
