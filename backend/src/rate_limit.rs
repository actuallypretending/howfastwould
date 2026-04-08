use dashmap::DashMap;
use std::net::IpAddr;
use std::time::Instant;

/// In-memory rate limiter: max_requests per window_secs, keyed by IP.
pub struct RateLimiter {
    map: DashMap<IpAddr, (u32, Instant)>,
    max_requests: u32,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            map: DashMap::new(),
            max_requests,
            window_secs,
        }
    }

    /// Returns Ok(()) if allowed, Err(seconds_until_reset) if rate-limited.
    pub fn check(&self, ip: IpAddr) -> Result<(), u64> {
        let now = Instant::now();
        let mut entry = self.map.entry(ip).or_insert((0, now));
        let (count, window_start) = entry.value_mut();

        if now.duration_since(*window_start).as_secs() >= self.window_secs {
            *count = 0;
            *window_start = now;
        }

        if *count >= self.max_requests {
            let elapsed = now.duration_since(*window_start).as_secs();
            let retry_after = self.window_secs.saturating_sub(elapsed);
            return Err(retry_after);
        }

        *count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_allows_under_limit() {
        let limiter = RateLimiter::new(3, 60);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_ok());
    }

    #[test]
    fn test_blocks_over_limit() {
        let limiter = RateLimiter::new(2, 60);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_err());
    }

    #[test]
    fn test_different_ips_independent() {
        let limiter = RateLimiter::new(1, 60);
        let ip1 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let ip2 = IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8));
        assert!(limiter.check(ip1).is_ok());
        assert!(limiter.check(ip2).is_ok());
        assert!(limiter.check(ip1).is_err());
        assert!(limiter.check(ip2).is_err());
    }
}
