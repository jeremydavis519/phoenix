#!/bin/sh

# TODO: Check every permutation of targets and available features.

PHOENIX_TARGET="aarch64/qemu-virt" \
PHOENIX_ASSEMBLER="aarch64-none-elf-as" \
PHOENIX_CPU="cortex-a53" \
PHOENIX_ARCHIVER="aarch64-none-elf-ar" \
cargo check -Z build-std --target=aarch64-phoenix-eabi --features=all_languages,self-test "$@" && \
cargo check -Z build-std --target=aarch64-phoenix-eabi --features=all_languages,unit-test "$@" && \
cargo check -Z build-std --target=aarch64-phoenix-eabi --features=all_languages,profiler "$@" && \
cargo check -Z build-std --target=aarch64-phoenix-eabi --features=all_languages "$@" && \
cargo check --features=all_languages,self-test "$@" && \
cargo check --features=all_languages,unit-test "$@" && \
cargo check --features=all_languages,profiler "$@" && \
cargo check --features=all_languages "$@"
