pub enum State {
    Uninitialized,
    Show(CitySelection),
}

#[derive(Debug)]
pub enum CitySelection {
    All,
    Ghosts,
    Selected(Vec<CityConstraint>),
}

#[derive(Debug)]
pub struct CityConstraint {}
