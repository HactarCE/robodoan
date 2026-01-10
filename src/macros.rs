/// Logging with indentation and verbosity filtering.
macro_rules! log {
    ($params:expr) => {
        log!($params, 0);
    };
    ($params:expr, $indent:expr $(, $($msg:tt)*)?) => {
        if $params.verbosity > $indent {
            #[allow(clippy::reversed_empty_ranges)]
            for _ in 0..$indent {
                print!("  ");
            }
            println!($($($msg)*)?);
        }
    };
}

macro_rules! overprint {
    ($($args:tt)*) => {
        print!("\r");
        print!($($args)*);
        ::std::io::Write::flush(&mut ::std::io::stdout()).unwrap();
    };
}

macro_rules! overprintln {
    ($($args:tt)*) => {
        print!("\r");
        println!($($args)*);
    };
}
