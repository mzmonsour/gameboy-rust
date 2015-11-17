# Gameboy Rust

A basic gameboy emulator written in Rust

A simple project to test my ability to write Rust code, and learn more about
hardware emulation by using a mostly well documented hardware set.

## Supported Features

- Tile and sprite based rendering
- Mostly complete CPU emulation
- V-Blank interrupt routines

## Currently unsupported features

- User input
- Audio
- Most other interrupt routines
- Various internal I/O ports
- Switchable ROM banks
- Good performance

## Usage

Run the command

````
$ gameboy-rust /path/to/rom
````

Curently, the emulator expects a working bootstrapper rom to reside in
`rom/bios.bin`. This is likely to change as the RST instruction is properly
emulated.

As well, the emulator will only correctly emulate cartridge type 0. Meaning
simple ROMs that contain only 32kB of memory, and no extra features such as
memory controllers, batteries, etc.

## Building from scratch

If Rust has been installed correctly, then building should be as simple as
running the following command.

````
$ cargo build
````

Similarly, if built with cargo, the emulator can be run by the following
command.

````
$ cargo run /path/to/rom
````
