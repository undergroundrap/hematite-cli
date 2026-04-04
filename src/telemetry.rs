use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref GLOBAL_COUNT: Mutex<i32> = Mutex::new(0);
}

pub fn risky_increment() {
    // BUG: Holding the lock across an await point or just a bad practice block
    let mut data = GLOBAL_COUNT.lock().unwrap();
    *data += 1;
    // Assume some long-running or complex logic here
    println!("Count is: {}", *data);
}
