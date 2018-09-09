
use multiboot2::BootInformation;
use ::memory::paging::entry::EntryFlags;
//use ::memory::paging::temporary_page::TemporaryPage;
use ::memory::paging::{Page, PageIter};
use ::memory::{Frame, FrameIter};
use ::memory::KERNEL_BASE;
use rlibc::memcpy;
use core::mem;
use ::interrupts::context::ExceptionStackFrame;

fn get_userprog_address(boot_info: &BootInformation) -> (usize, usize) {
    for module in boot_info.module_tags() {
        if module.name() == "userprog" {
            let start = module.start_address() as usize;
            let end = module.end_address() as usize;
            return (start, end);
        }
    }
    return (0,0);
}

pub fn start_process(boot_info: &BootInformation) {

    //let mut lock = MEMORY_CONTROLLER.lock();
    //let &mut MemoryController {
    //    ref mut active_table,
    //    ref mut frame_allocator,
    //    stack_allocator: _,
    //} = lock.as_mut().unwrap();
    //// 1. Get the new page table running

    //let mut temporary_page = 
    //    TemporaryPage::new(Page(0x9ff_ffff_fff), &mut frame_allocator); // magic #
	//let mut user_table = { 
    //    let frame = frame_allocator.allocate_frame()
    //        .expect("Out of memory when trying to create user process");
    //    InactivePageTable::new(frame, &mut active_table, &mut temporary_page, true);
    //};


    // 2. copy the code into an executable page
    let flags = EntryFlags::WRITABLE 
              | EntryFlags::USER_ACCESSIBLE;
    //    - can get start/end from boot_info

    let (section_start, section_end) = get_userprog_address(boot_info);
    let program_size = section_end - section_start;

    //let section_frame_range = FrameIter {
    //    start: Frame::containing_address(section_start),
    //    end: Frame::containing_address(section_end),
    //};

    //for frame in section_frame_range {
    //    ::memory::paging::identity_map(frame, flags);
    //}
   
    // copy program to new address
    let program_start: usize = 0x10_0000;
    
    let page_range = PageIter {
        start: Page::containing_address(program_start),
        end: Page::containing_address(program_start + program_size),
    };

    for page in page_range {
        page.map(flags)
    }

    let program_pointer = program_start as *mut u8;
    let section_pointer = section_start as *const u8;

    unsafe { memcpy(program_pointer, section_pointer, program_size) };

    let program_stack = ::memory::alloc_stack(1)
        .expect("Could not allocate stack for new process");
    let stack_pointer = program_stack.top();

    //let func_pointer = program_start as *const _;

    let exception_stack = ExceptionStackFrame {
        instruction_pointer: program_start,
        code_segment: 0b1111,
        cpu_flags: 0x202,
        stack_pointer: stack_pointer,
        stack_segment: 0,
    };

    let ex_pointer = &exception_stack as *const _;
    // switch to new table for good
    //let kernel_table = active_table.switch(user_table);
    //temporary_page.consume(&mut frame_allocator);

    // 4. add kernel stack to the tss
    //let process_stack = ::memory::alloc_stack(1)
    //    .expect("Could not allocate stack for process");
    //let mut tss = TSS.lock();
    //tss.privilege_stack_table[KERNEL_TSS_INDEX as usize] = 
    //    VirtualAddress(process_stack.top());

    // 5. Transmute the memory and jump to the code
    //    - currently in the lib.rs file

    //let program: unsafe extern "C" fn() = unsafe {
    //    mem::transmute(func_pointer)
    //};
    println!("{:x}", stack_pointer);
    unsafe {
        asm!("
            iretq" :: "{rsp}"(ex_pointer) :: "intel", "volatile")
    //asm!("
    //     push 0x0 
    //     push $0 
    //     push 0x202 
    //     push 100011b
    //     push $1
    //     iretq" :: "r"(stack_pointer), "r"(func_pointer) :: "intel", "volatile");
    }
}
