//! Token-bucket rate limiter partagé entre les threads de capture (story 2.5) — protège
//! CPU/disque en cas de pic de trafic. Paquets excédentaires droppés, jamais mis en attente
//! (un bucket bloquant retarderait la capture au lieu de la protéger).

use std::sync::Mutex;
use std::time::Instant;

pub struct TokenBucket {
    capacity: f64,
    rate_per_sec: f64,
    state: Mutex<BucketState>,
}

struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(rate_per_sec: u32) -> Self {
        let rate = f64::from(rate_per_sec.max(1));
        Self {
            capacity: rate,
            rate_per_sec: rate,
            state: Mutex::new(BucketState {
                tokens: rate,
                last_refill: Instant::now(),
            }),
        }
    }

    /// `true` si un jeton a pu être consommé (paquet retenu), `false` sinon (à dropper).
    pub fn try_acquire(&self) -> bool {
        let mut state = self.state.lock().expect("mutex token-bucket empoisonné");
        self.refill(&mut state);
        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&self, state: &mut BucketState) {
        let elapsed = state.last_refill.elapsed().as_secs_f64();
        state.tokens = (state.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        state.last_refill = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::TokenBucket;

    #[test]
    fn epuise_puis_refuse_au_dela_de_la_capacite() {
        let bucket = TokenBucket::new(2);
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire());
    }
}
