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

    pub fn new() -> TAPTestGroup {
        TAPTestGroup {
            count: 0,
        }
    }

    pub fn plan(&mut self) {
        println!("0..{}", self.count);
    }

    pub fn ok(&mut self, message: Option<&str>) {
        match message {
            Some(s) => println!("ok {}\n", s),
            None => println!("ok \n"),
        };
    }

    pub fn not_ok(&mut self, message: &str) {
        println!("not ok {}", message);
    }

    pub fn assert_tap(&mut self, cond: bool, message: &str) {
        if cond {
            self.ok(None);
        } else {
            self.not_ok(message);
        }
    }
}
