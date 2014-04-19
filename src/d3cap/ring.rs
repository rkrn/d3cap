// Example modified from https://github.com/mozilla/rust/issues/3562#issuecomment-9210203

// Fixed size buffer: when it is at capacity push will drop the oldest element.
// Demonstrates custom data structure, custom iteration, operator overloading, struct encapsulation.
// To run execute: rustc --test ring.rs && ./ring
extern crate std;

use std::iter::Iterator;
use std::fmt;
use std::fmt::{Show,Formatter};

// This contains the data that represents our ring buffer. In general only one
// allocation occurs: when the struct is first created and buffer is allocated.
// Copying a RingBuffer will cause a heap allocation but the compiler will
// warn us on attempts to copy it implicitly.
pub struct RingBuffer<T> {
    buffer: Vec<T>,
    capacity: uint,        // number of elements the buffer is able to hold (can't guarantee that vec capacity is exactly what we set it to)
    size: uint,            // number of elements with legit values in the buffer
    next: uint,            // index at which new elements land
}



// This is an impl which does not implement a trait: it merely provides some
// methods for our struct.
impl<T> RingBuffer<T> {
    pub fn new(capacity: uint) -> RingBuffer<T> {
        let mut ring = RingBuffer {buffer: Vec::new(), capacity: capacity, size: 0, next: 0};
        ring.buffer.reserve(capacity);
        ring
    }

    pub fn len(&self) -> uint {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn clear(&mut self) {
        self.buffer.truncate(0);
        self.size = 0;
        self.next = 0;
    }

    pub fn get<'a>(&'a self, i: uint) -> &'a T {
        assert!(i < self.size);

        if self.size < self.capacity {
            self.buffer.get(i)
        } else {
            self.buffer.get((self.next + i) % self.capacity)
        }
    }

    pub fn iter<'a>(&'a self) -> RingIterator<'a, T> {
        RingIterator { rb: self, i: 0 }
    }

    pub fn push(&mut self, element: T) {
        assert!(self.capacity > 0);

        if self.size < self.capacity {
            self.buffer.push(element);
            self.size += 1;
        } else {
            *self.buffer.get_mut(self.next) = element;
        }
        self.next = (self.next + 1) % self.capacity;
    }
}

// This is how rust handles operator overloading. Here we provide
// an implementation for ops::Index which allows users to subscript
// a RingBuffer using the [] operator.
// impl<T> ops::Index<uint, T> for RingBuffer<T> {
//     fn index(&self, index: &uint) -> T {
//         assert!(*index < self.size);

//         if self.size < self.capacity {
//             self.buffer[*index]
//         } else {
//             self.buffer[(self.next + *index) % self.capacity]
//         }
//     }
// }

pub struct RingIterator<'s, T> {
    rb: &'s RingBuffer<T>,
    i: uint
}

impl<'s, T> Iterator<&'s T> for RingIterator<'s, T> {
    fn next(&mut self) -> Option<&'s T> {
        if self.i < self.rb.size {
            let out = Some(self.rb.get(self.i));
            self.i += 1;
            out
        } else {
            None
        }
    }
}

// Users can always use the %? format specifier to display the full details of
// structs (and any other type). But because of the way that elements wrap
// around this can be confusing. Here we provide a to_str method that shows
// the elements in the same order as they appear to users.
impl<T: Show> Show for RingBuffer<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f.buf, "[");
        let mut first = true;
        for e in self.iter() {
            if !first {
                write!(f.buf, ",");
            }
            first = false;
            e.fmt(f);
        }
        write!(f.buf, "]")
    }
}

#[test]
fn test_basics() {
    // size 0
    let buffer: RingBuffer<int> = RingBuffer(0);    // rust type inference works very well, but not in this case
    assert!(buffer.len() == 0);

    // size 1
    let mut buffer = RingBuffer(1);
    assert!(buffer.len() == 0);

    buffer.push(2);
    assert!(buffer.len() == 1);
    assert!(*buffer.get(0) == 2);

    buffer.push(3);
    assert!(buffer.len() == 1);
    assert!(*buffer.get(0) == 3);

    // size 4
    let mut buffer = RingBuffer(4);
    assert!(buffer.len() == 0);

    buffer.push(1);
    assert!(buffer.len() == 1);
    assert!(*buffer.get(0) == 1);

    buffer.push(2);
    assert!(buffer.len() == 2);
    assert!(*buffer.get(0) == 1);
    assert!(*buffer.get(1) == 2);

    buffer.push(3);
    assert!(buffer.len() == 3);
    assert!(*buffer.get(0) == 1);
    assert!(*buffer.get(1) == 2);
    assert!(*buffer.get(2) == 3);

    buffer.push(4);
    assert!(buffer.len() == 4);
    assert!(*buffer.get(0) == 1);
    assert!(*buffer.get(1) == 2);
    assert!(*buffer.get(2) == 3);
    assert!(*buffer.get(3) == 4);

    // At this point the elements have wrapped around.
    buffer.push(5);
    assert!(buffer.len() == 4);
    assert!(*buffer.get(3) == 5);

    // But the public API hides this from clients (and the private fields
    // can only be used within this module).
    assert!(*buffer.get(0) == 2);
    assert!(*buffer.get(1) == 3);
    assert!(*buffer.get(2) == 4);
    assert!(*buffer.get(3) == 5);
    assert!(buffer.to_str() == ~"[2, 3, 4, 5]");

    // clear
    buffer.clear();
    assert!(buffer.len() == 0);

    buffer.push(2);
    assert!(buffer.len() == 1);
    assert!(*buffer.get(0) == 2);

    buffer.push(3);
    assert!(buffer.len() == 2);
    assert!(*buffer.get(0) == 2);
    assert!(*buffer.get(1) == 3);
}

// Rust uses a lot of functional programming idioms. One that takes some getting
// used to for imperative programmers is an avoidance of loops (loops rely on
// mutation of a loop variable which is not functional style). Instead looping is
// typically done with functions taking closures, the most common of which are:
// each, map, filter, and fold.
#[test]
fn test_functional() {
    let mut buffer: RingBuffer<int> = RingBuffer(4);
    buffer.push(1);
    buffer.push(3);
    buffer.push(5);
    buffer.push(2);

    // each calls a closure with each element
    // it is more functional than an explicit loop, but requires side effects in order to
    // do anything useful (because the closures user's give to each don't return values)
    let mut max = 0;
    for element in buffer {
        if *element > max {max = *element}    // dereference because each returns elements by reference
    }
    assert!(max == 5);

    let odd: ~[bool] = buffer.iter().map(|e| {*e & 1 == 1}).collect();
    assert!(odd == ~[true, true, true, false]);

    // filter returns elements for which the closure returns true
    let odd: ~[int] = buffer.iter().filter_map(|&e| {
        if e & 1 == 1 { Some(e) } else { None }
    }).collect();
    assert!(odd == ~[1, 3, 5]);

    // fold uses the closure to combine elements together (possibly into a different type)
    // either forwards (foldl) or in reverse (foldr)
    let sum: int = buffer.iter().fold(0, |a, &b| a + b);
    assert!(sum == 1 + 3 + 5 + 2);
}
