pub enum State {
    Uninitialized,
    Show(CitySelection),
}

pub enum CitySelection {
    All,
    Ghosts,
    Selected(Vec<CityConstraint>),
}

pub struct CityConstraint {}
