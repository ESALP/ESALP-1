
use multiboot2::BootInformation;

fn start_process(boot_info: &BootInformation) {

    // 1. Get the new page table running
    //    See unit test in memory/paging/table.rs
    //    - lock the memory controller
    //    - create new temporary page
    //    - clone higher half into
    //    - switch the p4 (see memory init)
    // 2. copy the code into an executable page
    //    - can get start/end from boot_info
    // 3. Do something with scheduler?????
    // 4. Set up the gdt / tss
    // 5. Transmute the memory and jump to the code
    //    - currently in the lib.rs file
}
