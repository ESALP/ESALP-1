// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use multiboot2::BootInformation;

use memory::{Frame, FrameAllocate, KERNEL_BASE};
use multiboot2::{MemoryAreaIter, MemoryArea};

/// An iterator acrossed physical frames using memory areas.
pub struct AreaFrameAllocator<'a> {
    next_free_frame: Frame,
    current_area: Option<&'static MemoryArea>,
    areas: MemoryAreaIter,
    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
    boot_info: &'a BootInformation,
}

impl<'a> AreaFrameAllocator<'a> {
    /// Returns a new `AreaFrameAllocator`
    pub fn new(kernel_start: usize,
               kernel_end: usize,
               multiboot_start: usize,
               multiboot_end: usize,
               boot_info: &BootInformation,
               memory_areas: MemoryAreaIter)
               -> AreaFrameAllocator {
        let mut alloc = AreaFrameAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            multiboot_start: Frame::containing_address(multiboot_start),
            multiboot_end: Frame::containing_address(multiboot_end),
            boot_info: boot_info, 
        };
        alloc.choose_next_area();
        alloc
    }

    /// Looks for free `Frame`s in the next memory area
    fn choose_next_area(&mut self) {
        self.current_area = self.areas
            .clone()
            .filter(|area| {
                let address = area.base_addr + area.length - 1;
                Frame::containing_address(address as usize) >= self.next_free_frame
            })
            .min_by_key(|area| area.base_addr);

        if let Some(area) = self.current_area {
            let start_frame = Frame::containing_address(area.base_addr as usize);
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }
}
    type Item = Frame;

impl<'a> FrameAllocate for AreaFrameAllocator<'a> {

    fn allocate_frame(&mut self) -> Option<Frame> {
        if let Some(area) = self.current_area {
            // "Clone the area to return it if it's free. Frame doesn't
            // implement Clone, but we can construct an identical frame
            let frame = Frame(self.next_free_frame.0);

            // The last frame of the current area
            let current_area_last_frame = {
                let address = area.base_addr + area.length - 1;
                Frame::containing_address(address as usize)
            };
            
            let contained_by = |frame: &Frame, low: &Frame, high: &Frame| -> bool {
               low <= frame && frame <= high 
            };

            if match frame {
                ref f if &current_area_last_frame < f 
                    // all frames of current area are used, switch to the new area
                    => {
                        self.choose_next_area();
                        false
                    },
                ref f if contained_by(f, &self.kernel_start, &self.kernel_end) 
                    // 'frame' is used by the kernel
                    => {
                        self.next_free_frame = Frame(self.kernel_end.0 + 1);
                        false
                    },
                ref f if contained_by(f, &self.multiboot_start, &self.multiboot_end)
                    // 'frame' is used by the multiboot information structure
                    => {
                        self.next_free_frame = Frame(self.multiboot_end.0 + 1);
                        false
                    },
                ref f => {
                    let mut unused = true;
                    for module in self.boot_info.module_tags() {
                        let start = module.start_address() as usize + KERNEL_BASE;
                        let end = module.end_address() as usize + KERNEL_BASE;
                        let startFrame = Frame::containing_address(start);
                        let endFrame = Frame::containing_address(end);
                        if contained_by(f, &startFrame, &endFrame) {
                            // 'frame' is used by the multiboot structure
                            self.next_free_frame = Frame(endFrame.0 + 1);
                            unused = false;
                            break;
                        }
                    }
                    unused
                },
            } {
                // hooray! we can allocate the frame!
                self.next_free_frame.0 += 1;
                return Some(frame);
            }
            
            // At this point, the frame has failed to allocate, recurse.
            self.allocate_frame()
    
        } else {
            None // no free frames
        }
    }
}
