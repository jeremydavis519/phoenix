#!/bin/sh
#
# Runs all of the tests for the whole git repository. The exit code is 0 if and only if all the
# tests pass.

exec find . -name Makefile -print0 | xargs -0 dirname -z | xargs -0L1 sh -c 'cd "$0" && make test'
