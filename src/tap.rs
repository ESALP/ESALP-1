// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms

use core::fmt::Write;

pub struct TAPTestGroup<'a, T: 'a + Write> {
    writer: &'a mut T,
    pub count: u8,
}

impl<'a, T> TAPTestGroup<'a, T> 
    where T: Write {

    pub fn new(w: &mut T) -> TAPTestGroup<T> {
        TAPTestGroup::<T> {
            writer: w,
            count: 0,
        }
    }

    pub fn plan(&mut self) {
        self.writer.write_fmt(format_args!("0..{}", self.count));
    }

    pub fn ok(&mut self, message: Option<&str>) {
        match message {
            Some(s) => self.writer.write_fmt(format_args!("ok {}\n", s)),
            None => self.writer.write_str("ok \n"),
        };
    }

    pub fn not_ok(&mut self, message: &str) {
        self.writer.write_fmt(format_args!("not ok {}", message));
    }

    pub fn assert_tap(&mut self, cond: bool, message: &str) {
        if cond {
            self.ok(None);
        } else {
            self.not_ok(message);
        }
    }
}
