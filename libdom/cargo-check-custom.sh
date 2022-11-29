#!/bin/sh

export RUSTFLAGS="--cfg procmacro2_semver_exempt"

cargo check "$@"
