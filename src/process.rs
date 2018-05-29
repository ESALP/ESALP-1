// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use memory::{MemoryController,MEMORY_CONTROLLER,FrameAllocate,KERNEL_BASE};
use memory::paging::{Page, InactivePageTable};
use memory::paging::temporary_page::TemporaryPage;


pub fn create_process() {
    // Step 1 - create new page directory
    let mut lock = MEMORY_CONTROLLER.lock();
    let &mut MemoryController {
        ref mut active_table,
        ref mut frame_allocator,
        stack_allocator: _,
    } = lock.as_mut().unwrap();

    let frame = frame_allocator.allocate_frame().expect("Unable to allocate frame");

    // random unmapped address
    let addr = 4096 * 512 * 512 * 12; 
    let page = Page::containing_address(addr);
    //assert!(active_table.mapper.translate(addr).is_none(), "Chose bad page");
    let mut temp_page = TemporaryPage::new(page, frame_allocator);

    let inactive_table = InactivePageTable::new(frame, active_table, &mut temp_page);
    println!("Inactive Table created!");

    // Step 2 - copy the kernel over
    {
        let table = temp_page.map_table_frame(inactive_table.p4_frame, active_table);

        let mut i: usize = 0;
        for entry in unsafe{ active_table.mapper.p4.as_ref().entries.iter()} {
            if KERNEL_BASE as u64 <= entry.0 {
                //let new_entry = (*entry).clone()
                table[i] = (*entry).clone();
            }
            i += 1
        }
    }
    println!("Kernel copied");

}
