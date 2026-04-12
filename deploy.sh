#!/usr/bin/env bash
set -e

echo ":: clippy"
cargo clippy --all-targets -- -D warnings

echo ":: build"
cargo build

echo ":: deploy"
cp target/debug/breathe.exe ~/bin/breathe.exe

echo ":: done"
