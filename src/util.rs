pub trait SetMinMax {
    fn setmin(&mut self, other: Self) -> bool;
    fn setmax(&mut self, other: Self) -> bool;
}

impl<T: PartialOrd> SetMinMax for T {
    fn setmin(&mut self, other: Self) -> bool {
        if *self > other {
            *self = other;
            true
        } else {
            false
        }
    }

    fn setmax(&mut self, other: Self) -> bool {
        if *self < other {
            *self = other;
            true
        } else {
            false
        }
    }
}
