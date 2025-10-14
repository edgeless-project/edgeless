// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::num::Wrapping;

// Parameters from glib's implementation.
const MODULUS: Wrapping<u32> = Wrapping(2147483648);
const MULTIPLIER: Wrapping<u32> = Wrapping(1103515245);
const OFFSET: Wrapping<u32> = Wrapping(12345);

pub struct Lcg {
    seed: Wrapping<u32>,
}

impl Lcg {
    pub fn new(seed: u32) -> Self {
        Self {
            seed: Wrapping(seed),
        }
    }

    pub fn rand(&mut self) -> f32 {
        self.seed = (MULTIPLIER * self.seed + OFFSET) % MODULUS;
        self.seed.0 as f32 / MODULUS.0 as f32
    }
}

pub fn random_matrix(lcg: &mut Lcg, size: usize) -> Vec<f32> {
    let mut new_matrix = vec![0.0; size * size];
    for value in new_matrix.iter_mut() {
        *value = lcg.rand();
    }
    new_matrix
}

pub fn random_vector(lcg: &mut Lcg, size: usize) -> Vec<f32> {
    let mut new_vector = vec![0.0; size];
    for value in new_vector.iter_mut() {
        *value = lcg.rand();
    }
    new_vector
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_lcg_rand() {
        let mut numbers = std::collections::HashSet::new();
        let mut lcg = Lcg::new(42);
        for _ in 0..1000 {
            let rnd = lcg.rand();
            numbers.insert((rnd * 20.0).floor() as u32);
        }
        assert_eq!(20, numbers.len());
    }

    #[test]
    fn test_lcg_random_matrix() {
        let mut lcg = Lcg::new(42);
        let matrix = random_matrix(&mut lcg, 1000);
        assert_eq!(1000 * 1000, matrix.len());
        assert_ne!(0.0_f32, matrix.iter().sum());
    }
}
