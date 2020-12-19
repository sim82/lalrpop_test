#!/bin/sh


cargo run --bin compiler < $1 | cargo run --bin assembler | cargo run --bin vm