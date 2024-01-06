use bevy::prelude::*;

// https://en.wikipedia.org/wiki/Linear_congruential_generator
#[derive(Resource, Clone, Copy, Reflect, Debug)]
pub struct Rng {
    x: u64, // seed
    m: u64,
    a: u64,
    c: u64,
}

impl Default for Rng {
    fn default() -> Self {
        Self {
            x: 1024,
            m: 2_147_483_647,
            a: 48_271,
            c: 0,
        }
    }
}

#[allow(dead_code)]
impl Rng {
    pub fn new(seed: u64) -> Self {
        Self {
            x: seed,
            ..default()
        }
    }

    pub fn next_f64(&mut self) -> f64 {
        self.x = (self.a * self.x + self.c) % self.m;
        self.x as f64 / self.m as f64
    }

    pub fn next_f32(&mut self) -> f32 {
        self.next_f64() as f32
    }

    /// returns an int in the range [min, max)
    pub fn next_i32(&mut self, min: i32, max: i32) -> i32 {
        let r: f32 = self.next_f32();
        let t = r * (max - min) as f32;
        t as i32 + min
    }

    pub fn next_usize(&mut self, min: usize, max: usize) -> usize {
        let r: f64 = self.next_f64();
        let t = r * (max - min) as f64;
        t as usize + min
    }

    /// remove a random element from the vector and return it
    pub fn extract_random<T>(&mut self, c: &mut Vec<T>) -> T {
        let n = self.next_usize(0, c.len());
        c.swap_remove(n)
    }
}

#[test]
fn evenly_distributed() {
    let mut avg = 0f64;
    let mut rng = Rng::new(1028);
    let count = 1000000u32;
    for _ in 0..count {
        avg += rng.next_f64();
    }
    let epsilon = 0.001f64;
    avg /= count as f64;
    assert!(0.5 - epsilon < avg && avg < 0.5 + epsilon);
}
