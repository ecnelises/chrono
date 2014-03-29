/*!
 * ISO 8601 time.
 */

use std::fmt;
use duration::Duration;

pub trait Timelike {
    /// Returns the hour number from 0 to 23.
    fn hour(&self) -> uint;

    /// Returns the hour number from 1 to 12 with a boolean flag,
    /// which is false for AM and true for PM.
    #[inline]
    fn hour12(&self) -> (bool, uint) {
        let hour = self.hour();
        let mut hour12 = hour % 12;
        if hour12 == 0 { hour12 = 12; }
        (hour >= 12, hour12)
    }

    /// Returns the minute number from 0 to 59.
    fn minute(&self) -> uint;

    /// Returns the second number from 0 to 59.
    fn second(&self) -> uint;

    /// Returns the number of nanoseconds since the whole non-leap second.
    /// The range from 1,000,000,000 to 1,999,999,999 represents the leap second.
    fn nanosecond(&self) -> uint;

    /// Makes a new value with the hour number changed.
    ///
    /// Returns `None` when the resulting value would be invalid.
    fn with_hour(&self, hour: uint) -> Option<Self>;

    /// Makes a new value with the minute number changed.
    ///
    /// Returns `None` when the resulting value would be invalid.
    fn with_minute(&self, min: uint) -> Option<Self>;

    /// Makes a new value with the second number changed.
    ///
    /// Returns `None` when the resulting value would be invalid.
    fn with_second(&self, sec: uint) -> Option<Self>;

    /// Makes a new value with nanoseconds since the whole non-leap second changed.
    ///
    /// Returns `None` when the resulting value would be invalid.
    fn with_nanosecond(&self, nano: uint) -> Option<Self>;

    /// Returns the number of non-leap seconds past the last midnight.
    #[inline]
    fn nseconds_from_midnight(&self) -> uint {
        self.hour() * 3600 + self.minute() * 60 + self.second()
    }
}

/// ISO 8601 time without timezone.
/// Allows for the nanosecond precision and optional leap second representation.
#[deriving(Eq, TotalEq, Ord, TotalOrd, Hash)]
pub struct TimeZ {
    priv hour: u8,
    priv min: u8,
    priv sec: u8,
    priv frac: u32,
}

impl TimeZ {
    /// Makes a new `TimeZ` from hour, minute and second.
    ///
    /// Returns `None` on invalid hour, minute and/or second.
    #[inline]
    pub fn from_hms(hour: uint, min: uint, sec: uint) -> Option<TimeZ> {
        TimeZ::from_hms_nano(hour, min, sec, 0)
    }

    /// Makes a new `TimeZ` from hour, minute, second and millisecond.
    /// The millisecond part can exceed 1,000 in order to represent the leap second.
    ///
    /// Returns `None` on invalid hour, minute, second and/or millisecond.
    #[inline]
    pub fn from_hms_milli(hour: uint, min: uint, sec: uint, milli: uint) -> Option<TimeZ> {
        TimeZ::from_hms_nano(hour, min, sec, milli * 1_000_000)
    }

    /// Makes a new `TimeZ` from hour, minute, second and microsecond.
    /// The microsecond part can exceed 1,000,000 in order to represent the leap second.
    ///
    /// Returns `None` on invalid hour, minute, second and/or microsecond.
    #[inline]
    pub fn from_hms_micro(hour: uint, min: uint, sec: uint, micro: uint) -> Option<TimeZ> {
        TimeZ::from_hms_nano(hour, min, sec, micro * 1_000)
    }

    /// Makes a new `TimeZ` from hour, minute, second and nanosecond.
    /// The nanosecond part can exceed 1,000,000,000 in order to represent the leap second.
    ///
    /// Returns `None` on invalid hour, minute, second and/or nanosecond.
    pub fn from_hms_nano(hour: uint, min: uint, sec: uint, nano: uint) -> Option<TimeZ> {
        if hour >= 24 || min >= 60 || sec >= 60 || nano >= 2_000_000_000 { return None; }
        Some(TimeZ { hour: hour as u8, min: min as u8, sec: sec as u8, frac: nano as u32 })
    }
}

impl Timelike for TimeZ {
    #[inline] fn hour(&self) -> uint { self.hour as uint }
    #[inline] fn minute(&self) -> uint { self.min as uint }
    #[inline] fn second(&self) -> uint { self.sec as uint }
    #[inline] fn nanosecond(&self) -> uint { self.frac as uint }

    #[inline]
    fn with_hour(&self, hour: uint) -> Option<TimeZ> {
        if hour >= 24 { return None; }
        Some(TimeZ { hour: hour as u8, ..*self })
    }

    #[inline]
    fn with_minute(&self, min: uint) -> Option<TimeZ> {
        if min >= 60 { return None; }
        Some(TimeZ { min: min as u8, ..*self })
    }

    #[inline]
    fn with_second(&self, sec: uint) -> Option<TimeZ> {
        if sec >= 60 { return None; }
        Some(TimeZ { sec: sec as u8, ..*self })
    }

    #[inline]
    fn with_nanosecond(&self, nano: uint) -> Option<TimeZ> {
        if nano >= 2_000_000_000 { return None; }
        Some(TimeZ { frac: nano as u32, ..*self })
    }
}

impl Add<Duration,TimeZ> for TimeZ {
    fn add(&self, rhs: &Duration) -> TimeZ {
        let mut secs = self.nseconds_from_midnight() as int + rhs.nseconds() as int;
        let mut nanos = self.frac + rhs.nnanoseconds() as u32;

        // always ignore leap seconds after the current whole second
        let maxnanos = if self.frac >= 1_000_000_000 {2_000_000_000} else {1_000_000_000};

        if nanos >= maxnanos {
            nanos -= maxnanos;
            secs += 1;
        }
        let (s, mins) = (secs % 60, secs / 60);
        let (m, hours) = (mins % 60, mins / 60);
        let h = hours % 24;
        TimeZ { hour: h as u8, min: m as u8, sec: s as u8, frac: nanos }
    }
}

/*
// Rust issue #7590, the current coherence checker can't handle multiple Add impls
impl Add<TimeZ,TimeZ> for Duration {
    #[inline]
    fn add(&self, rhs: &TimeZ) -> TimeZ { rhs.add(self) }
}
*/

impl Sub<TimeZ,Duration> for TimeZ {
    fn sub(&self, rhs: &TimeZ) -> Duration {
        // the number of whole non-leap seconds
        let secs = (self.hour as int - rhs.hour as int) * 3600 +
                   (self.min  as int - rhs.min  as int) * 60 +
                   (self.sec  as int - rhs.sec  as int) - 1;

        // the fractional second from the rhs to the next non-leap second
        let maxnanos = if rhs.frac >= 1_000_000_000 {2_000_000_000} else {1_000_000_000};
        let nanos1 = maxnanos - rhs.frac;

        // the fractional second from the last leap or non-leap second to the lhs
        let lastfrac = if self.frac >= 1_000_000_000 {1_000_000_000} else {0};
        let nanos2 = self.frac - lastfrac;

        Duration::seconds(secs) + Duration::nanoseconds(nanos1 as int + nanos2 as int)
    }
}

impl fmt::Show for TimeZ {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (sec, nano) = if self.frac >= 1_000_000_000 {
            (self.sec + 1, self.frac - 1_000_000_000)
        } else {
            (self.sec, self.frac)
        };

        try!(write!(f.buf, "{:02}:{:02}:{:02}", self.hour, self.min, sec));
        if nano == 0 {
            Ok(())
        } else if nano % 1_000_000 == 0 {
            write!(f.buf, ",{:03}", nano / 1_000_000)
        } else if nano % 1_000 == 0 {
            write!(f.buf, ",{:06}", nano / 1_000)
        } else {
            write!(f.buf, ",{:09}", nano)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use duration::Duration;

    fn hmsm(hour: uint, min: uint, sec: uint, millis: uint) -> TimeZ {
        TimeZ::from_hms_milli(hour, min, sec, millis).unwrap()
    }

    #[test]
    fn test_time_add() {
        fn check(lhs: TimeZ, rhs: Duration, sum: TimeZ) {
            assert_eq!(lhs + rhs, sum);
            //assert_eq!(rhs + lhs, sum);
        }

        check(hmsm(3, 5, 7, 900), Duration::zero(), hmsm(3, 5, 7, 900));
        check(hmsm(3, 5, 7, 900), Duration::milliseconds(100), hmsm(3, 5, 8, 0));
        check(hmsm(3, 5, 7, 1_300), Duration::milliseconds(800), hmsm(3, 5, 8, 100));
        check(hmsm(3, 5, 7, 900), Duration::seconds(86399), hmsm(3, 5, 6, 900)); // overwrap
        check(hmsm(3, 5, 7, 900), Duration::seconds(-86399), hmsm(3, 5, 8, 900));
        check(hmsm(3, 5, 7, 900), Duration::days(12345), hmsm(3, 5, 7, 900));
    }

    #[test]
    fn test_time_sub() {
        fn check(lhs: TimeZ, rhs: TimeZ, diff: Duration) {
            // `time1 - time2 = duration` is equivalent to `time2 - time1 = -duration`
            assert_eq!(lhs - rhs, diff);
            assert_eq!(rhs - lhs, -diff);
        }

        check(hmsm(3, 5, 7, 900), hmsm(3, 5, 7, 900), Duration::zero());
        check(hmsm(3, 5, 7, 900), hmsm(3, 5, 7, 600), Duration::milliseconds(300));
        check(hmsm(3, 5, 7, 200), hmsm(2, 4, 6, 200), Duration::seconds(3600 + 60 + 1));
        check(hmsm(3, 5, 7, 200), hmsm(2, 4, 6, 300),
                   Duration::seconds(3600 + 60) + Duration::milliseconds(900));

        // treats the leap second as if it coincides with the prior non-leap second,
        // as required by `time1 - time2 = duration` and `time2 - time1 = -duration` equivalence.
        check(hmsm(3, 5, 7, 200), hmsm(3, 5, 6, 1_800), Duration::milliseconds(400));
        check(hmsm(3, 5, 7, 1_200), hmsm(3, 5, 6, 1_800), Duration::milliseconds(400));
        check(hmsm(3, 5, 7, 1_200), hmsm(3, 5, 6, 800), Duration::milliseconds(400));

        // additional equality: `time1 + duration = time2` is equivalent to
        // `time2 - time1 = duration` IF AND ONLY IF `time2` represents a non-leap second.
        assert_eq!(hmsm(3, 5, 6, 800) + Duration::milliseconds(400), hmsm(3, 5, 7, 200));
        assert_eq!(hmsm(3, 5, 6, 1_800) + Duration::milliseconds(400), hmsm(3, 5, 7, 200));
    }

    #[test]
    fn test_time_fmt() {
        assert_eq!(hmsm(23, 59, 59,   999).to_str(), ~"23:59:59,999");
        assert_eq!(hmsm(23, 59, 59, 1_000).to_str(), ~"23:59:60");
        assert_eq!(hmsm(23, 59, 59, 1_001).to_str(), ~"23:59:60,001");
        assert_eq!(TimeZ::from_hms_micro(0, 0, 0, 43210).unwrap().to_str(), ~"00:00:00,043210");
        assert_eq!(TimeZ::from_hms_nano(0, 0, 0, 6543210).unwrap().to_str(), ~"00:00:00,006543210");
    }
}
