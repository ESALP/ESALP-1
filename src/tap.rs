// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms

pub struct TAPTestGroup {
    pub count: u8,
}

impl TAPTestGroup {

    pub fn new(c: u8) -> TAPTestGroup {
        TAPTestGroup {
            count: c,
        }
    }

    pub fn plan(&self) {
        serial_println!("0..{}", self.count);
    }

    pub fn ok(&self, message: Option<&str>) {
        match message {
            Some(s) => serial_println!("ok {}\n", s),
            None => serial_println!("ok \n"),
        };
    }

    pub fn not_ok(&self, message: &str) {
        serial_println!("not ok {}", message);
    }

    pub fn assert_tap(&self, cond: bool, message: &str) {
        if cond {
            self.ok(None);
        } else {
            self.not_ok(message);
        }
    }
}
