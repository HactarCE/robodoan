use std::{cell::Cell, sync::Mutex};

use crate::sim::{Twist, blockbuilding::BlockList};

static LIST: Mutex<Vec<(BlockList, Twist)>> = Mutex::new(vec![]);

static SAMPLE_RATE: usize = 1_00;

thread_local! {
    static COUNT: Cell<usize> = Cell::new(0);
}

pub fn add_example(block: &BlockList, twist: Twist) {
    let count = COUNT.get();
    COUNT.set(count + 1);
    if count % SAMPLE_RATE == 0 {
        LIST.lock().unwrap().push((block.clone(), twist));
    }
}

pub fn take_all_examples() -> Vec<(BlockList, Twist)> {
    std::mem::take(&mut LIST.lock().unwrap())
}
