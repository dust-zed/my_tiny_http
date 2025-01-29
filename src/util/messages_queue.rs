use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

enum Control<T> {
    Elem(T),
    Unblock,
}

pub struct MessageQueue<T>
where
    T: Send,
{
    queue: Mutex<VecDeque<Control<T>>>,
    condvar: Condvar,
}

impl<T> MessageQueue<T>
where
    T: Send,
{
    pub fn with_capacity(capacity: usize) -> Arc<MessageQueue<T>> {
        Arc::new(MessageQueue {
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            condvar: Condvar::new(),
        })
    }

    ///pushes and element to the queue
    pub fn push(&self, value: T) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(Control::Elem(value));
        self.condvar.notify_one();
    }
}
