# ROM Files

Retro64 ships with **minimal ROM stubs** that let the CPU boot into a sane
state (KERNAL reset vector + trivial idle loop), but not enough to launch
BASIC or run the corpus under `tests/corpus/basic/`. To use the emulator
for anything beyond the CPU / chip test suite you need the real Commodore
64 ROMs.

Place these three files in this directory:

- `kernal`  — 8192 bytes, KERNAL ROM
- `basic`   — 8192 bytes, BASIC V2 ROM
- `chargen` — 4096 bytes, Character Generator ROM

Then run the emulator with: `retro64 --rom-dir ./roms`

ROM images can be extracted from real C64 hardware, obtained from VICE's
`C64` subdirectory, or fetched from the MEGA65 Open-ROMs project (BASIC
Open-ROM is a partial BASIC 2.0 replacement that is legally redistributable).
None of these files are committed to this repository.
