//! Versi rilis `MAJOR.MINOR.PATCH` (tag `v0.4.0`). Pre-release tidak ditawarkan.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// `"0.4.0"` atau `"v0.4.0"`; pre-release/build (`-rc.1`, `+x`) dan bentuk lain → `None`.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let s = s.strip_prefix('v').unwrap_or(s);
        let mut parts = s.split('.');
        let mut next = || -> Option<u32> {
            let p = parts.next()?;
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            p.parse().ok()
        };
        let v = Self {
            major: next()?,
            minor: next()?,
            patch: next()?,
        };
        parts.next().is_none().then_some(v)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_and_without_prefix() {
        let v = Version::parse("v0.4.0").unwrap();
        assert_eq!(v, Version::parse("0.4.0").unwrap());
        assert_eq!(v.to_string(), "0.4.0");
    }

    #[test]
    fn rejects_pre_release_and_garbage() {
        for s in [
            "0.4.0-rc.1",
            "0.4",
            "0.4.0.1",
            "v",
            "",
            "a.b.c",
            "1..2",
            "+1.0.0",
        ] {
            assert_eq!(Version::parse(s), None, "{s}");
        }
    }

    #[test]
    fn orders_numerically() {
        let p = |s| Version::parse(s).unwrap();
        assert!(p("0.10.0") > p("0.9.9"));
        assert!(p("1.0.0") > p("0.99.99"));
        assert!(p("0.4.1") > p("0.4.0"));
    }
}
