# Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
# See the README.md file at the top-level directory of this
# distribution.
#
# Licensed under the MIT license <LICENSE or
# http://opensource.org/licenses/MIT>, at your option.
# This file may not be copied, modified, or distributed
# except according to those terms.

arch ?= x86_64
name ?= ESALP
target ?= $(arch)-ESALP
rust_os := target/$(target)/debug/lib$(name).a
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso
module := src/arch/$(arch)/module

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
# We need this to find all rust files. GNU make doesn't support recursive
# wildcards, so this function supplies it for us. (from SO)
rwildcard=$(wildcard $1$2) $(foreach d,$(wildcard $1*),$(call rwildcard,$d/,$2))
rust_source_files := $(call rwildcard,src/,*.rs)
assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, \
	build/arch/$(arch)/%.o, $(assembly_source_files))

.PHONY: all clean run iso debug test

all: $(kernel)

clean:
	@xargo clean
	@rm -r build
	@rm -r test || true

qflags := -s -serial stdio

cargo_flags :=

ifeq ($(int),yes)
	qflags += -d int
endif
ifdef display
	qflags += -display $(display)
endif
ifeq ($(kvm),yes)
	qflags += -enable-kvm
endif
ifeq ($(reboot),no)
	qflags += --no-reboot
endif
ifndef kbmap
	kbmap := us
endif

binutils_prefix :=

ifeq ($(cross),yes)
	binutils_prefix = x86_64-elf-
endif	

ld := $(binutils_prefix)ld

run: $(iso)
	@qemu-system-x86_64 $(qflags) -cdrom $(iso)

debug: $(iso)
	@qemu-system-x86_64 $(qflags) -cdrom $(iso) -S

test: cargo_flags += --features test
test: $(iso)
	@qemu-system-x86_64 $(qflags) -cdrom $(iso) -display none \
		-device isa-debug-exit,iobase=0xf4,iosize=0x04

iso: $(iso)

$(iso): $(kernel) build/arch/$(arch)/$(kbmap).bin $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@cp build/arch/$(arch)/$(kbmap).bin build/isofiles/boot/keyboard
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles
	
# Rust image
$(kernel): $(rust_os) $(assembly_object_files) $(linker_script)
	@$(ld) -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) $(rust_os)

# Rust static lib
$(rust_os): export RUST_TARGET_PATH=$(shell pwd)
$(rust_os): $(rust_source_files)
	@xargo build --target $(target) $(cargo_flags)

# Keyboard maps
build/arch/$(arch)/%.bin: $(module)/keyboard/%.asm
	@nasm $< -o $@

# compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@
