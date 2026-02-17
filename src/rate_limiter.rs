use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{self, FromRequest};
use rocket::Request;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct WindowCounter {
    count: u32,
    window_start: Instant,
}

pub struct RateLimiter {
    requests_per_window: u32,
    window_duration: Duration,
    max_entries: usize,
    counters: Mutex<HashMap<IpAddr, WindowCounter>>,
    call_counter: AtomicU32,
}

const CLEANUP_INTERVAL: u32 = 1000;
const DEFAULT_MAX_ENTRIES: usize = 100_000;

impl RateLimiter {
    pub fn new(requests_per_window: u32, window_duration: Duration) -> Self {
        RateLimiter {
            requests_per_window,
            window_duration,
            max_entries: DEFAULT_MAX_ENTRIES,
            counters: Mutex::new(HashMap::new()),
            call_counter: AtomicU32::new(0),
        }
    }

    pub fn check_rate_limit(&self, ip: IpAddr) -> bool {
        if self.requests_per_window == 0 {
            return true;
        }

        let now = Instant::now();
        let Ok(mut counters) = self.counters.lock() else {
            return true;
        };

        if counters.len() >= self.max_entries && !counters.contains_key(&ip) {
            counters.retain(|_, v| now.duration_since(v.window_start) < self.window_duration);
            if counters.len() >= self.max_entries {
                return false;
            }
        }

        let counter = counters.entry(ip).or_insert(WindowCounter {
            count: 0,
            window_start: now,
        });

        if now.duration_since(counter.window_start) >= self.window_duration {
            counter.count = 0;
            counter.window_start = now;
        }

        counter.count += 1;
        let allowed = counter.count <= self.requests_per_window;

        drop(counters);

        if self.call_counter.fetch_add(1, Ordering::Relaxed).is_multiple_of(CLEANUP_INTERVAL) {
            self.cleanup_expired(now);
        }

        allowed
    }

    fn cleanup_expired(&self, now: Instant) {
        let Ok(mut counters) = self.counters.lock() else {
            return;
        };
        counters.retain(|_, v| now.duration_since(v.window_start) < self.window_duration);
    }
}

pub struct RateLimited;

/// Cached result of rate limit check, stored as request-local state.
/// This prevents double-counting when Rocket forwards between ranked routes.
struct RateLimitResult(bool);

#[rocket::async_trait]
impl<'a> FromRequest<'a> for RateLimited {
    type Error = ();

    async fn from_request(req: &'a Request<'_>) -> request::Outcome<Self, Self::Error> {
        let allowed = req.local_cache(|| {
            let rate_limiter = match req.rocket().state::<RateLimiter>() {
                Some(rl) => rl,
                None => return RateLimitResult(true),
            };

            let ip = match req.remote() {
                Some(addr) => addr.ip(),
                None => return RateLimitResult(true),
            };

            RateLimitResult(rate_limiter.check_rate_limit(ip))
        });

        if allowed.0 {
            Outcome::Success(RateLimited)
        } else {
            Outcome::Error((Status::TooManyRequests, ()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn test_ip(last_octet: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, last_octet))
    }

    #[test]
    fn within_limit() {
        let rl = RateLimiter::new(5, Duration::from_secs(60));
        let ip = test_ip(1);
        for _ in 0..5 {
            assert!(rl.check_rate_limit(ip));
        }
    }

    #[test]
    fn over_limit() {
        let rl = RateLimiter::new(3, Duration::from_secs(60));
        let ip = test_ip(1);
        for _ in 0..3 {
            assert!(rl.check_rate_limit(ip));
        }
        assert!(!rl.check_rate_limit(ip));
        assert!(!rl.check_rate_limit(ip));
    }

    #[test]
    fn separate_ips_have_independent_limits() {
        let rl = RateLimiter::new(2, Duration::from_secs(60));
        let ip1 = test_ip(1);
        let ip2 = test_ip(2);

        assert!(rl.check_rate_limit(ip1));
        assert!(rl.check_rate_limit(ip1));
        assert!(!rl.check_rate_limit(ip1));

        assert!(rl.check_rate_limit(ip2));
        assert!(rl.check_rate_limit(ip2));
        assert!(!rl.check_rate_limit(ip2));
    }

    #[test]
    fn window_reset() {
        let rl = RateLimiter::new(2, Duration::from_millis(50));
        let ip = test_ip(1);

        assert!(rl.check_rate_limit(ip));
        assert!(rl.check_rate_limit(ip));
        assert!(!rl.check_rate_limit(ip));

        std::thread::sleep(Duration::from_millis(60));

        assert!(rl.check_rate_limit(ip));
        assert!(rl.check_rate_limit(ip));
        assert!(!rl.check_rate_limit(ip));
    }

    #[test]
    fn zero_disables_rate_limiting() {
        let rl = RateLimiter::new(0, Duration::from_secs(60));
        let ip = test_ip(1);
        for _ in 0..1000 {
            assert!(rl.check_rate_limit(ip));
        }
    }

    #[test]
    fn cleanup_removes_expired_entries() {
        let rl = RateLimiter::new(1, Duration::from_millis(50));
        let ip = test_ip(1);

        assert!(rl.check_rate_limit(ip));

        std::thread::sleep(Duration::from_millis(60));

        let now = Instant::now();
        rl.cleanup_expired(now);

        let counters = rl.counters.lock().unwrap();
        assert!(counters.is_empty());
    }

    #[test]
    fn max_entries_rejects_new_ips_when_full() {
        let mut rl = RateLimiter::new(10, Duration::from_secs(60));
        rl.max_entries = 3;

        assert!(rl.check_rate_limit(test_ip(1)));
        assert!(rl.check_rate_limit(test_ip(2)));
        assert!(rl.check_rate_limit(test_ip(3)));
        assert!(!rl.check_rate_limit(test_ip(4)));
    }

    #[test]
    fn max_entries_allows_existing_ips() {
        let mut rl = RateLimiter::new(10, Duration::from_secs(60));
        rl.max_entries = 2;

        assert!(rl.check_rate_limit(test_ip(1)));
        assert!(rl.check_rate_limit(test_ip(2)));
        // Existing IP still allowed even at capacity
        assert!(rl.check_rate_limit(test_ip(1)));
    }

    #[test]
    fn max_entries_evicts_expired_to_make_room() {
        let mut rl = RateLimiter::new(10, Duration::from_millis(50));
        rl.max_entries = 2;

        assert!(rl.check_rate_limit(test_ip(1)));
        assert!(rl.check_rate_limit(test_ip(2)));

        std::thread::sleep(Duration::from_millis(60));

        // Expired entries are cleaned up to make room
        assert!(rl.check_rate_limit(test_ip(3)));
    }
}
