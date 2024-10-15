// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::SeedableRng;
use rand_distr::Distribution;

/// Arrival model type
enum ArrivalType {
    /// Inter-arrival between consecutive workflows and durations are exponentially distributed.
    Poisson,
    /// One new workflow arrive every new inter-arrival time.
    Incremental,
    /// Add workflows incrementally until the warm up period finishes, then keep until the end of the experiment.
    IncrAndKeep,
    /// Add a single workflow.
    Single,
}

/// Arrival model, which determines the interarrival and lifetime of workflows.
pub struct ArrivalModel {
    arrival_type: ArrivalType,
    warmup: u64,
    duration: u64,
    interarrival: u64,
    lifetime: u64,
    rng: rand_pcg::Lcg128Xsl64,
    interarrival_exp_rv: rand_distr::Exp<f64>,
    lifetime_exp_rv: rand_distr::Exp<f64>,
    counter: u64,
}

impl ArrivalModel {
    ///
    ///  Create a new ArrivalModel
    ///
    /// Parameters:
    /// - `arrival_type`: a string representing the arrival model type of choice
    /// - `warmup`: the warm-up duration, in fractional seconds
    /// - `duration`: the experiment duration, in fractional seconds
    /// - `seed`: pseudo-random number generator seed
    /// - `interarrival`: average interarrival, in seconds, used by some arrival types
    /// - `lifetime`: average lifetime, in seconds, used by some arrival types
    ///
    pub fn new(arrival_type: &str, warmup: f64, duration: f64, seed: u64, interarrival: f64, lifetime: f64) -> anyhow::Result<Self> {
        anyhow::ensure!(duration > 0.0, "cannot have negative experiment duration");

        let arrival_type = match arrival_type.to_ascii_lowercase().as_str() {
            "poisson" => {
                anyhow::ensure!(lifetime > 0.0, "the average lifetime cannot be negative");
                anyhow::ensure!(interarrival > 0.0, "the average interarrival cannot be negative");
                ArrivalType::Poisson
            }
            "incremental" => ArrivalType::Incremental,
            "incr-and-keep" => ArrivalType::IncrAndKeep,
            "single" => ArrivalType::Single,
            _ => anyhow::bail!("invalid arrival model type {}: ", arrival_type),
        };

        let rng = rand_pcg::Pcg64::seed_from_u64(seed);
        let interarrival_exp_rv = rand_distr::Exp::new(1.0 / interarrival)?;
        let lifetime_exp_rv = rand_distr::Exp::new(1.0 / lifetime)?;

        Ok(Self {
            arrival_type,
            warmup: crate::utils::to_microseconds(warmup),
            duration: crate::utils::to_microseconds(duration),
            interarrival: crate::utils::to_microseconds(interarrival),
            lifetime: crate::utils::to_microseconds(lifetime),
            rng,
            interarrival_exp_rv,
            lifetime_exp_rv,
            counter: 0,
        })
    }

    /// Return the next arrival time and lifetime, in microseconds.
    pub fn next(&mut self, now: u64) -> Option<(u64, u64)> {
        self.counter += 1;
        let next_periodic = self.interarrival * (self.counter - 1);
        let arrival_time = match self.arrival_type {
            ArrivalType::Poisson => now + crate::utils::to_microseconds(self.interarrival_exp_rv.sample(&mut self.rng)),
            ArrivalType::Incremental => next_periodic,
            ArrivalType::IncrAndKeep => {
                if next_periodic < self.warmup {
                    next_periodic
                } else {
                    return None;
                }
            }
            ArrivalType::Single => {
                if self.counter == 1 {
                    0_u64
                } else {
                    return None;
                }
            }
        };
        let lifetime = match self.arrival_type {
            ArrivalType::Poisson => arrival_time + crate::utils::to_microseconds(self.lifetime_exp_rv.sample(&mut self.rng)),
            ArrivalType::Incremental => arrival_time + self.lifetime,
            ArrivalType::IncrAndKeep | ArrivalType::Single => self.duration,
        };
        Some((arrival_time, lifetime))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_arrival_model_invalid() {
        assert!(ArrivalModel::new("", 0.0, 100.0, 42, 10.0, 1.0).is_err());
        assert!(ArrivalModel::new("invalid", 0.0, 100.0, 42, 10.0, 1.0).is_err());
        assert!(ArrivalModel::new("poisson", 0.0, -100.0, 42, 10.0, 1.0).is_err());
    }

    #[test]
    fn test_arrival_model_poisson() {
        assert!(ArrivalModel::new("poisson", 0.0, 100.0, 42, -10.0, 1.0).is_err());
        assert!(ArrivalModel::new("poisson", 0.0, 100.0, 42, 10.0, -1.0).is_err());

        let mut model = ArrivalModel::new("poisson", 0.0, 100.0, 42, 1.0, 10.0).unwrap();

        let mut interarrival_sum = 0.0;
        let mut lifetime_sum = 0.0;
        for now in 0..10000 {
            let (arrival_time, end_time) = model.next(now).unwrap();
            assert!(arrival_time > now);
            assert!(end_time > arrival_time);
            interarrival_sum += (arrival_time - now) as f64;
            lifetime_sum += (end_time - arrival_time) as f64;
        }
        assert_eq!(1, crate::utils::to_seconds((interarrival_sum / 10000 as f64) as u64).round() as u64);
        assert_eq!(10, crate::utils::to_seconds((lifetime_sum / 10000 as f64) as u64).round() as u64);
    }

    #[test]
    fn test_arrival_model_incremental() {
        let mut model = ArrivalModel::new("incremental", 0.0, 100.0, 42, 1.0, 10.0).unwrap();

        for i in 0..10 {
            let (arrival_time, end_time) = model.next(0_u64).unwrap();
            assert_eq!(crate::utils::to_microseconds(1.0 * i as f64), arrival_time);
            assert_eq!(crate::utils::to_microseconds(1.0 * i as f64 + 10.0), end_time);
        }
    }

    #[test]
    fn test_arrival_model_incr_and_keep() {
        let mut model = ArrivalModel::new("incr-and-keep", 50.0, 100.0, 42, 1.0, 10.0).unwrap();

        for i in 0..100 {
            let now = crate::utils::to_microseconds(i as f64);
            if now < crate::utils::to_microseconds(50.0) {
                let (arrival_time, end_time) = model.next(0_u64).unwrap();
                assert_eq!(crate::utils::to_microseconds(1.0 * i as f64), arrival_time);
                assert_eq!(crate::utils::to_microseconds(100.0), end_time);
            } else {
                assert!(model.next(0_u64).is_none());
            }
        }
    }

    #[test]
    fn test_arrival_model_single() {
        let mut model = ArrivalModel::new("single", 0.0, 100.0, 42, 1.0, 10.0).unwrap();

        let now = 42;
        let (arrival_time, end_time) = model.next(now).unwrap();
        assert_eq!(crate::utils::to_microseconds(0.0), arrival_time);
        assert_eq!(crate::utils::to_microseconds(100.0), end_time);

        assert!(model.next(0_u64).is_none());
    }
}
