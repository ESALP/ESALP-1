file ./build/kernel-x86_64.bin
target remote localhost:1234
set architecture i386:x86-64
set disassembly-flavor intel

display /i $pc

define longfix
    disconnect
    set architecture i386:x86-64:intel
    target remote localhost:1234
end
