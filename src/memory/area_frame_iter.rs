// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use memory::Frame;
use multiboot2::{MemoryAreaIter, MemoryArea};

/// An iterator acrossed physical frames using memory areas.
pub struct AreaFrameIter {
    next_free_frame: Frame,
    current_area: Option<&'static MemoryArea>,
    areas: MemoryAreaIter,
    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

impl AreaFrameIter {
    /// Returns a new `AreaFrameIter`
    pub fn new(kernel_start: usize,
               kernel_end: usize,
               multiboot_start: usize,
               multiboot_end: usize,
               memory_areas: MemoryAreaIter)
               -> AreaFrameIter {
        let mut iter = AreaFrameIter {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            multiboot_start: Frame::containing_address(multiboot_start),
            multiboot_end: Frame::containing_address(multiboot_end),
        };
        iter.choose_next_area();
        iter
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
impl Iterator for AreaFrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        if let Some(area) = self.current_area {
            // "Clone the area to return it if it's free. Frame doesn't
            // implement Clone, but we can construct an identical frame
            let frame = Frame(self.next_free_frame.0);

            // The last frame of the current area
            let current_area_last_frame = {
                let address = area.base_addr + area.length - 1;
                Frame::containing_address(address as usize)
            };

            if frame > current_area_last_frame {
                // all frames of current area are used, switch to the new area
                self.choose_next_area();
            } else if frame >= self.kernel_start && frame <= self.kernel_end {
                // 'frame' is used by the kernel
                self.next_free_frame = Frame(self.kernel_end.0 + 1)
            } else if frame >= self.multiboot_start && frame <= self.multiboot_end {
                // 'frame' is used by the multiboot information structure
                self.next_free_frame = Frame(self.multiboot_end.0 + 1)
            } else {
                // frame is unused, increment 'next_free_frame' and return it
                self.next_free_frame.0 += 1;
                return Some(frame);
            }
            // 'frame' was not valid, try again with the new 'next_free_frame'
            self.next()
        } else {
            None // no free frames
        }
    }
}
