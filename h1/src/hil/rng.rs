// Copyright 2018 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Interfaces for accessing a random number generator.
//!
//! A random number generator produces a stream of random numbers, either from
//! hardware or based on an initial seed. The [RNG](trait.RNG.html) trait
//! provides a simple, implementation agnostic interface for getting new random
//! values.
//!
//! The interface is designed to work well with random number generators that
//! may not have values ready immediately. This is important when generating
//! numbers from a low-bandwidth hardware random number generator or when the
//! RNG is virtualized among many consumers.
//!
//! Random numbers are yielded to the [Client](trait.Client.html) as an
//! `Iterator` which only terminates when no more numbers are currently
//! available. Clients can request more randmoness if needed and will be called
//! again when more is available.
//!
//! # Example
//!
//! The following example is a simple capsule that prints out a random number
//! once a second using the `Alarm` and `RNG` traits.
//!
//! ```
//! struct RngTest<'a, A: Alarm + 'a> {
//!     rng: &'a RNG,
//!     alarm: &'a A
//! }
//!
//! impl<'a, A: Alarm> RngTest<'a, A> {
//!     pub fn initialize(&self) {
//!         let interval = 1 * <A::Frequency>::frequency();
//!         let tics = self.alarm.now().wrapping_add(interval);
//!         self.alarm.set_alarm(tics);
//!     }
//! }
//!
//! impl<'a, A: Alarm> time::Client for RngTest<'a, A> {
//!     fn fired(&self) {
//!         self.rng.get();
//!     }
//! }
//!
//! impl<'a, A: Alarm> rng::Client for RngTest<'a, A> {
//!     fn randomness_available(&self, randomness: &mut Iterator<Item = u32>) -> rng::Continue {
//!         match randomness.next() {
//!             Some(random) => {
//!                 println!("Rand {}", random);
//!                 let interval = 1 * <A::Frequency>::frequency();
//!                 let tics = self.alarm.now().wrapping_add(interval);
//!                 self.alarm.set_alarm(tics);
//!                 rng::Continue::Done
//!             },
//!             None => rng::Continue::More
//!         }
//!     }
//! }
//! ```

/// Denotes whether the [Client](trait.Client.html) wants to be notified when
/// `More` randomness is available or if they are `Done`
#[derive(Debug, Eq, PartialEq)]
pub enum Continue {
    /// More randomness is required.
    More,
    /// No more randomness required.
    Done,
}

/// Generic interface for a random number generator
///
/// Implementors should assume the client implements the
/// [Client](trait.Client.html) trait.
pub trait RNG<'a> {
    /// Set the client for this random number generator.
    /// THe client will be called in response to requests for randomness.
    fn set_client(&self, client: &'a dyn Client);

    /// Initiate acquiring a random number.
    ///
    /// The implementor may ignore this command if the generation proccess is
    /// already in progress.
    fn get(&self);
}

/// An [RNG](trait.RNG.html) client
///
/// Clients of an [RNG](trait.RNG.html) must implement this trait.
pub trait Client {
    /// Called by the (RNG)[trait.RNG.html] when there are one or more random
    /// numbers available
    ///
    /// `randomness` in an `Iterator` of available random numbers. The amount of
    /// randomness available may increase if `randomness` is not consumed
    /// quickly so clients should not rely on iterator termination to finish
    /// consuming random numbers.
    ///
    /// The client returns either `Continue::More` if the iterator did not have
    /// enough random values and the client would like to be called again when
    /// more is available, or `Continue::Done`.
    fn randomness_available(&self, randomness: &mut dyn Iterator<Item = u32>) -> Continue;
}
