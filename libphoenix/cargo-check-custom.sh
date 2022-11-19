#!/bin/sh

# TODO: Check every permutation of targets and available features.

cargo check -Z build-std --target=aarch64-phoenix --features=global-allocator "$@" && \
cargo check -Z build-std --target=aarch64-phoenix --features=kernelspace "$@" && \
cargo check -Z build-std --target=aarch64-phoenix --features=no-start "$@" && \
cargo check -Z build-std --target=aarch64-phoenix --features=profiler "$@" && \
cargo check -Z build-std --target=aarch64-phoenix "$@" && \
cargo check --features=profiler && \
cargo check
