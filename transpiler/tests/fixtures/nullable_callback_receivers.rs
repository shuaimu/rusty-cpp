use std::collections::VecDeque;
use std::sync::Mutex;

type Callback = Option<Box<dyn FnMut(&mut i32)>>;
type Entries = VecDeque<Entry>;

struct Reply {
    callback: Callback,
}

impl Reply {
    fn invoke(&mut self, value: &mut i32) -> bool {
        let taken = self.callback.take();
        if taken.is_none() {
            return false;
        }
        let mut callback = taken.unwrap();
        callback(value);
        true
    }

    fn discard(&mut self) -> bool {
        self.callback.take().is_none()
    }
}

struct Entry {
    callback: Callback,
}

fn invoke_callback(callback: Callback, value: &mut i32) {
    let mut callback = callback.unwrap();
    callback(value);
}

struct Queue {
    entries: Mutex<Entries>,
}

impl Queue {
    fn invoke_front(&self, value: &mut i32) -> usize {
        let mut guard = self.entries.lock().unwrap();
        let initial_len = guard.len();
        let mut calls = 0usize;
        for _ in 0..initial_len {
            let mut entry = guard.pop_front().unwrap();
            if entry.callback.is_some() {
                invoke_callback(entry.callback.take(), value);
                calls += 1;
            }
        }
        calls
    }

    fn invoke_back(&self, value: &mut i32) -> bool {
        let mut guard = self.entries.lock().unwrap();
        let mut entry = guard.pop_back().unwrap();
        if entry.callback.is_none() {
            return false;
        }
        invoke_callback(entry.callback.take(), value);
        true
    }
}

pub fn check_receiver_inference() -> i32 {
    let increment = Box::new(7);
    let mut reply = Reply {
        callback: Some(Box::new(move |value: &mut i32| *value += *increment)),
    };
    let mut value = 0;
    if !reply.invoke(&mut value) || value != 7 {
        return 1;
    }
    if reply.invoke(&mut value) || value != 7 {
        return 2;
    }
    if !reply.callback.is_none() {
        return 3;
    }

    let mut discarded = Reply {
        callback: Some(Box::new(|value: &mut i32| *value += 1000)),
    };
    if discarded.discard() || !discarded.discard() {
        return 4;
    }

    let mut taken_by_function = Reply {
        callback: Some(Box::new(|value: &mut i32| *value += 2)),
    };
    let taken = std::mem::take(&mut taken_by_function.callback);
    if taken.is_none() || taken_by_function.callback.is_some() {
        return 5;
    }
    let mut callback = taken.unwrap();
    callback(&mut value);
    if value != 9 {
        return 6;
    }
    if !core::mem::take(&mut taken_by_function.callback).is_none() {
        return 7;
    }

    let queue = Queue {
        entries: Mutex::new(VecDeque::<Entry>::new()),
    };
    {
        let mut guard = queue.entries.lock().unwrap();
        guard.push_back(Entry {
            callback: Some(Box::new(|value: &mut i32| *value += 3)),
        });
        guard.push_back(Entry { callback: None });
        guard.push_back(Entry {
            callback: Some(Box::new(|value: &mut i32| *value += 5)),
        });
    }
    if queue.invoke_front(&mut value) != 2 || value != 17 {
        return 8;
    }
    if queue.invoke_front(&mut value) != 0 || value != 17 {
        return 9;
    }
    {
        let mut guard = queue.entries.lock().unwrap();
        guard.push_back(Entry { callback: None });
        guard.push_back(Entry {
            callback: Some(Box::new(|value: &mut i32| *value += 11)),
        });
    }
    if !queue.invoke_back(&mut value) || value != 28 {
        return 10;
    }
    if queue.invoke_back(&mut value) || value != 28 {
        return 11;
    }
    0
}
