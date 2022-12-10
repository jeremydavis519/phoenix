#!/bin/sh

# Cargo was for some reason not passing RUST_TARGET_PATH to rustc when running `cargo check` on a
# "bin" crate. This wrapper works around that bug.

RUST_TARGET_PATH=`dirname "$0"` $@
