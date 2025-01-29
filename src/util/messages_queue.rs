use std::sync::{Condvar, Mutex};
use std::collections::VecDeque;

enum Control<T> {
    Elem(T),
    Unblock,
}

pub struct MessageQueue<T> where T : Send {
    queue: Mutex<VecDeque<T>>,
    condvar: Condvar
}