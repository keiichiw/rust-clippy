// edition:2018
// run-rustfix
// aux-build:order_of_use_helper.rs

#![warn(clippy::order_of_use)]
#![allow(unused)]
#![allow(unused_imports)]

use crate::myself::*;
use order_of_use_helper::a::{
    b1::{c2, *},
    b2::c1,
};
use std::io;
use {order_of_use_helper::*, std::fs};

mod myself {
    pub fn a() {}
    pub fn b() {}
    pub fn c() {}
}

extern crate order_of_use_helper;

fn main() {
    // test code goes here
}
