# Test Corpus

This directory contains test programs for verifying the Retro64 emulator.

## Directories

- `cpu/` — CPU test suites (Klaus Dormann 6502 tests, etc.)
- `basic/` — BASIC programs for testing the interpreter
- `graphics/` — VIC-II graphics mode tests
- `sound/` — SID audio tests
- `storage/` — Storage format test files (.D64, .TAP, .T64)
- `demos/` — Public domain C64 demos

## BASIC Programs

The `.bas` files contain BASIC source code. The `.prg` files are tokenized
versions ready to load into the emulator.

To create .prg files from BASIC source, use petcat (from VICE):
```
petcat -w2 -o program.prg -- program.bas
```
