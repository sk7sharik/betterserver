use crate::map::Map;

pub(crate) struct RavineMist
{

}

impl Map for RavineMist
{
    fn name(&self) -> &str {
        "Ravine Mist"
    }
    
    fn index(&self) -> usize {
        1
    }
}

impl RavineMist
{
    pub fn new() -> RavineMist {
        RavineMist { }
    }
}