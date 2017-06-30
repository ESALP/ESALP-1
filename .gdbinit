file ./build/kernel-x86_64.bin
target remote localhost:1234
set architecture i386:x86-64
set disassembly-flavor intel

define longfix
    disconnect
    set architecture i386:x86-64:intel
    target remote localhost:1234
end

define page_table
    if $argc > 3
        echo Too many args\n
    end

    set $addr = 18446743522877890560
    set $i = 0
    while $i < $argc
        eval "set $addr = ($addr * 512) + ($arg%d * 4096)", $i
        set $i = $i + 1
    end
    set $addr = $addr | 0xffff800000000000
    p/x *($addr as &u64)@512
end
