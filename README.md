# ESALP [![Build Status](https://travis-ci.org/ESALP/ESALP-1.svg?branch=master)](https://travis-ci.org/ESALP/ESALP-1)
**E**ndangered **S**oaring **A**frican **L**ynx-**P**idgeon is a **work in progress** OS by Calvin Lee and JJ Garzella

It is an implimentation of Phillipp Oppermann's [Blog OS](https://github.com/phil-opp/blog_os), go check out that repository and [his blog](http://os.phil-opp.com/) for more details.

Project Goals:
+ No C environment
 + ESALP will be written in Rust (and some Assembler of course)
+ Flexibility
+ **More to come**


## Features
Right now it doesn't do much, but more is added every day!

Current features:
+ Can print to the VGA text buffer (in 255 different colors!)
+ Simple PS/2 keyboard driver
 + with multiple keyboard maps
+ Simple paging
 + With physical frame allocation _and_ deallocation
+ Kernel space heap
+ **More to come**

## How to Compile
1. Use `make all` to build ESALP
2. `make run` and you're done!

### Notes:
+ If your system binutils is not x86_64-elf format, you need to cross-compile binutils. By adding `cross=yes` to both make commands, the prefix `x86_64-elf-` will be added to all binutils commands.
+ `int=yes` prints out the registers on an interrupt and `reboot=no` stops qemu from rebooting. If you're stuck in an infinite reboot loop, `make run int=yes reboot=no` could be helpful
+ If kvm is your thing, run with `kvm=yes`

## Licensing
This code is licensed under the MIT license. See LICENSE for more details.
