#!/usr/bin/env bash

rm -rf encode decode

rustc -o encode src/encode.rs
rustc -o decode src/decode.rs
