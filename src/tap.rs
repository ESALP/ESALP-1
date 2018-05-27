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
    count: u8,
    cur: u8,
}

impl TestGroup {
    pub fn new(count: u8) -> TestGroup {
        let tap = TestGroup {
            count: count,
            cur: 0,
        };
        tap.plan();
        tap
    }

    fn plan(&self) {
        serial_println!("1..{}", self.count);
    }

    pub fn ok(&mut self, message: Option<&str>) {
        self.cur += 1;
        match message {
            Some(s) => serial_println!("ok {} - {}", self.cur, s),
            None => serial_println!("ok {}", self.cur),
        };
        assert!(self.cur <= self.count);
    }

    pub fn not_ok(&mut self, message: &str) {
        self.cur += 1;
        serial_println!("not ok {} - {}", self.cur, message);
        assert!(self.cur <= self.count);
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
        // HACK: split tests
        self.diagnostic("EOT");
    }
}
