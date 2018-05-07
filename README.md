# ESALP [![Build Status](https://travis-ci.org/ESALP/ESALP-1.svg?branch=master)](https://travis-ci.org/ESALP/ESALP-1)
**E**ndangered **S**oaring **A**frican **L**ynx-**P**idgeon is a **work in progress** OS by Calvin Lee and JJ Garzella

It is an implimentation of Phillipp Oppermann's [Blog OS](https://github.com/phil-opp/blog_os), go check out that repository and [his blog](http://os.phil-opp.com/) for more details.

## Project Goals
+ No C environment
  + ESALP will be written in Rust (and some Assembler of course)
+ Flexibility
+ **More to come**


## Features
Right now it doesn't do much, but more is added every day!

Current features:
+ Interaction
  + Can print to the VGA text buffer (in 255 different colors!)
  + Simple PS/2 keyboard driver
    + with multiple keyboard maps
  + Can print to the serial bus
+ Memory
  + Simple paging
    + With physical frame allocation _and_ deallocation
  + Kernel space heap
+ **More to come**

## How to Compile
1. Install packages, ESALP requires `xargo`, `nasm`, `grub-mkrescue`, `ld`, and a unix environment to build. To run, use `qemu`.
2. Use `make all` to build ESALP
3. `make run` and you're done!

### Compilation on macOS
ESALP also supports compilation on macOS. As macOS uses mostly Apple's infrastructure rather than linux's, it requires a bit more setup.
#### Compile Assembly on macOS
1. Homebrew
	- gcc
	- autoconf
	- xorriso
	- nasm
2. MacPorts
	- libmpc
	- gmp
	- mpfr
* 2.5. Optional: Compile libiconv
3. Cross-compile Binutils
	- use phil's page: http://os.phil-opp.com/cross-compile-binutils.html
	- include option --program-prefix
4. Cross-compile GCC
	- use OSDev page: http://wiki.osdev.org/GCC_Cross-Compiler
	- include option --program-prefix
5. Compile grub
	- use OSDev page on grub: http://wiki.osdev.org/GRUB#Installing_GRUB2_on_Mac_OS_X
	- put in platform-specific tools
#### Set up Rust on macOS
1. Install rustup
2. Get the proper nightly build in your folder
3. Homebrew
	- cmake
	- openssl
4. put a symlink to openssl in /usr/local/include
	- https://solitum.net/openssl-os-x-el-capitan-and-brew/
5. Install xargo
	- "cargo install xargo"
#### Running on macOS
+ Because we cross-compiled binutils earlier, we need to use `make all cross=yes` and `make run cross=yes`

### Notes:
+ If your system binutils is not x86_64-elf format, for example in macOS (see above), you need to cross-compile binutils. By adding `cross=yes` to both make commands, the prefix `x86_64-elf-` will be added to all binutils commands.
+ `int=yes` prints out the registers on an interrupt and `reboot=no` stops qemu from rebooting. If you're stuck in an infinite reboot loop, `make run int=yes reboot=no` could be helpful
+ If kvm is your thing, run with `kvm=yes`

## Licensing
This code is licensed under the MIT license. See LICENSE for more details.
