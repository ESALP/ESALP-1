
use multiboot2::BootInformation;
use ::memory::paging::entry::*;
use ::memory::{Page, PageIter, KERNEL_BASE};
use ::interrupts::TSS;

const KERNEL_TSS_INDEX: usize = 0;

fn get_userprog_address(boot_info: &BootInformation) (usize, usize) {
    for module in boot_info.module_tags() {
        if module.name() == "userprog" {
            let start = module.start_address() as usize + KERNEL_BASE;
            let end = module.end_address() as usize + KERNEL_BASE;
            return (start, end);
        }
    }
}

pub fn start_process(boot_info: &BootInformation) {

    let mut lock = MEMORY_CONTROLLER.lock();
    let &mut MemoryController {
        ref mut active_table,
        ref mut frame_allocator,
        stack_allocator: _,
    } = lock.as_mut().unwrap();
    // 1. Get the new page table running

    let mut temporary_page = 
        TemporaryPage::new(Page(0x9ff_ffff_fff), &mut allocator); // magic #
	let mut user_table = { 
        let frame = frame_allocator.allocate_frame()
            .expect("Out of memory when trying to create user process");
        InactivePageTable::new(frame, &mut active_table, &mut temporary_page, true);
    }


    // 2. copy the code into an executable page
    let flags = EntryFlags::WRITABLE 
              | EntryFlags::USER_ACCESSIBLE;
    //    - can get start/end from boot_info

    let (section_start, section_end) = get_userprog_address(boot_info);
    let program_size = end - start;
    
    let program_start = 0x100_0000_0000;
    
    let page_range = PageIter {
        start: Page::containing_address(program_start),
        end: Page::containing_addres(program_start + program_size),
    }

    active_table.with(&mut user_table, &mut temporary_page, |mapper| {
        for page in page_range {
            let old_page = 
                Page::containing_address(page - program_start + section_start);
            let frame = mapper.translate_page(old_page)
                .except("Could not translate page at {}", old_page.start_address());
            mapper.map_to(page, frame, flags, &mut frame_allocator)
                .expect("Could not map page to {}", page.start_address());
        }
    }

    // switch to new table for good
    let kernel_table = active_table.switch(user_table);
    temporary_page.consume(&mut frame_allocator);

    // 4. add kernel stack to the tss
    let process_stack = memory::alloc_stack(1);
    let tss = TSS.lock();
    tss.privilege_stack_table[KERNEL_TSS_INDEX] = process_stack;

    // 5. Transmute the memory and jump to the code
    //    - currently in the lib.rs file

    let func_pointer = program_start as *const ();
    let program: unsafe extern "C" fn() = unsafe {
        core::mem::transmute(func_pointer);
    }
    unsafe { func() };
}
