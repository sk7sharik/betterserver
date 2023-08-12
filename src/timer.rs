use std::{collections::HashMap, ops::SubAssign};
use std::hash::Hash;

pub(crate) struct Timer<T>
{
    pub frames: HashMap<T, u16>
}

impl<T> Timer<T>
where 
    T: Eq,
    T: Hash,
    T: Copy
{
    pub fn new() -> Timer<T>
    {
        Timer { frames: HashMap::new() }
    }

    pub fn tick(&mut self) 
    {
        for frame in self.frames.values_mut() {
            if *frame == 0 {
                continue;
            }

            frame.sub_assign(1);
        }
    }

    pub fn get(&mut self, key: T) -> u16
    {
        if !self.frames.contains_key(&key) {
            self.frames.insert(key, 0);
        }

        self.frames[&key]
    }

    pub fn set(&mut self, key: T, value: u16) {
        self.frames.insert(key, value);
    }
}