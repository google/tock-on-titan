//! Interfaces for accessing a random number generator.

#[derive(Eq, PartialEq)]
pub enum Continue {
    More,
    Done,
}

pub trait Rng {
    fn get_data(&self);
}

pub trait RngClient {
    fn random_data_available(&self, &mut Iterator<Item = u32>) -> Continue;
}
