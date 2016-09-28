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
target ?= $(arch)-unknown-none-gnu
rust_os := target/$(target)/debug/lib$(name).a
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso
module := src/arch/$(arch)/module

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, \
	build/arch/$(arch)/%.o, $(assembly_source_files))

.PHONY: all clean run iso

all: $(kernel)

clean:
	@xargo clean
	@rm -r build

qflags := -s

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

iso: $(iso)

$(iso): $(kernel) build/arch/$(arch)/$(kbmap).bin $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@cp build/arch/$(arch)/$(kbmap).bin build/isofiles/boot/keyboard
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles
	
$(kernel): xargo $(rust_os) $(assembly_object_files) $(linker_script)
	@$(ld) -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) $(rust_os)

xargo:
	@xargo build --target $(target)

build/arch/$(arch)/$(kbmap).bin: $(module)/keyboard/$(kbmap).asm
	@nasm $(module)/keyboard/$(kbmap).asm -o build/arch/$(arch)/$(kbmap).bin

# compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@
