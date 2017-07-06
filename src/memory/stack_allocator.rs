// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use memory::{PAGE_SIZE, FrameAllocate};
use memory::paging::{self, Page, PageIter, ActivePageTable};

#[derive(Debug)]
pub struct Stack {
    top: usize,
    bottom: usize,
}

impl Stack {
    fn new(top: usize, bottom: usize) -> Stack {
        assert!(top > bottom);

        Stack {
            top: top,
            bottom: bottom,
        }
    }

    pub fn top(&self) -> usize {
        self.top
    }

    pub fn bottom(&self) -> usize {
        self.bottom
    }
}

pub struct StackAllocator {
    range: PageIter,
}

impl StackAllocator {
    pub fn new(page_range: PageIter) -> StackAllocator {
        StackAllocator { range: page_range }
    }

    pub fn alloc_stack<FA>(&mut self,
                           active_table: &mut ActivePageTable,
                           allocator: &mut FA,
                           size: usize) -> Option<Stack>
        where FA: FrameAllocate
    {
        // Only mutate in success
        let mut range = self.range.clone();

        let guard_page = range.next();
        let stack_start = range.next();
        let stack_end = match size {
            0 => return None, /* Don't do anything for a zero sized stack */
            1 => stack_start,
            n => range.nth(n - 2),
        };

        match (guard_page, stack_start, stack_end) {
            (Some(_), Some(start), Some(end)) => {
                // Success, mutate and return
                self.range = range;

                // Map to physical pages
                for page in Page::range_inclusive(start, end) {
                    active_table.map(page, paging::WRITABLE, allocator);
                }

                // Create stack and return
                //
                // The stack grows downward
                let stack_top = end.start_address() + PAGE_SIZE;
                let stack_bottom = start.start_address();

                Some(Stack::new(stack_top, stack_bottom))
            },
            _ => None, /* Not enough pages */
        }
    }
}
