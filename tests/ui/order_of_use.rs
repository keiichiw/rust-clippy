#![warn(clippy::order_of_use)]


mod x {
    pub mod y {}
}
mod z {}

mod testing {
    mod base {
        pub mod hoge {}
        pub mod fuga {
            pub mod a {}
            pub mod b {}
        }
        pub mod test {
            pub mod test2 {}
        }
    }

    use base::hoge;

    use base::fuga::{a, b};
    use base::test::*;
    use std::path;
    use crate::x;
    use crate::x::y;
    use crate::z;
}

fn main() {
    // test code goes here
}
