use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;

pub trait Logger {
    fn write(&mut self, data: &[u8]);
}

pub struct Disk {
    file: Rc<RefCell<File>>,
}

impl Logger for Disk {
    fn write(&mut self, data: &[u8]) {
        todo!()
    }
}

pub struct BlackHole {}

impl Logger for BlackHole {
    fn write(&mut self, data: &[u8]) {
        todo!()
    }
}
