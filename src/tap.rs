// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms

use spin::Mutex;

pub struct TAPTestGroup {
    pub count: u8,
}

pub static GLOBAL_TEST_GROUP: Mutex<TAPTestGroup> = Mutex::new(TAPTestGroup::new());

impl TAPTestGroup {

    pub const fn new() -> TAPTestGroup {
        TAPTestGroup {
            count: 0,
        }
    }

    pub fn plan(&self) {
        serial_println!("0..{}", self.count);
    }

    pub fn ok(&mut self, message: Option<&str>) {
        match message {
            Some(s) => serial_println!("ok {} - {}", self.count, s),
            None => serial_println!("ok {}", self.count),
        };
        self.count += 1;
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
}
