use crate::map::Map;

pub(crate) struct HideAndSeek2
{

}

impl Map for HideAndSeek2
{
    fn name(&self) -> &str {
        "Hide and Seek 2"
    }

    fn index(&self) -> usize {
        0
    }

    fn ring_count(&self) -> usize {
        25
    }
}

impl HideAndSeek2
{
    pub fn new() -> HideAndSeek2 {
        HideAndSeek2 { }
    }
}