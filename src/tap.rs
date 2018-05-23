// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms

use core::ops::Drop;

pub struct TestGroup {
    pub count: u8,
}

impl TestGroup {
    pub const fn new() -> TestGroup {
        TestGroup {
            count: 0,
        }
    }

    fn plan(&self) {
        serial_println!("1..{}", self.count);
    }

    pub fn ok(&mut self, message: Option<&str>) {
        self.count += 1;
        match message {
            Some(s) => serial_println!("ok {} - {}", self.count, s),
            None => serial_println!("ok {}", self.count),
        };
    }

    pub fn not_ok(&mut self, message: &str) {
        serial_println!("not ok {} - {}", self.count, message);
        self.count += 1;
    }

    pub fn assert_tap(&mut self, cond: bool, message: &str) {
        if cond {
            self.ok(None);
        } else {
            self.not_ok(message);
        }
    }

    pub fn diagnostic(&self, msg: &str) {
        serial_println!("# {}", msg);
    }

}

impl Drop for TestGroup {
    fn drop(&mut self) {
        self.plan()
    }
}
