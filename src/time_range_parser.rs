use regex::Regex;
use std::collections::HashMap;

pub struct TimeRangeParser {
    regex_ranges: Regex,
    regex_range: Regex,
    regex_24h: Regex,
    regex_12h: Regex,
    variables: HashMap<String, u32>,
}

/// A time-range is a tuple of two timestamps, the first one is the start, the second one is the end.
/// The timestamps are represented as minutes since midnight.
pub type TimeRange = (u32, u32);

/// Utility function to convert hours to minutes
/// # Examples
///
/// ```
/// assert_eq!(h(10), 600);
/// ```
fn h(hours: u32) -> u32 {
    hours * 60
}

impl TimeRangeParser {
    pub fn new() -> TimeRangeParser {
        TimeRangeParser {
            regex_ranges: Regex::new(r"\((?<values>.*?)\)").unwrap(),
            regex_range: Regex::new(r"^(?<from>.*?)-(?<to>.*?)$").unwrap(),
            regex_24h: Regex::new(r"^(?<value>\d{1,2}(:\d{2})?)h$").unwrap(),
            regex_12h: Regex::new(r"^(?<value>\d{1,2}(:\d{2})?)(?<format>AM|PM)$").unwrap(),
            variables: HashMap::new(),
        }
    }

    /// Defines variables that can be used within time-ranges
    /// # Examples
    /// ```
    /// let mut parser = TimeRangeParser::new();
    ///
    /// parser.define_variables(
    ///   vec![
    ///     ("sunrise", h(6)),
    ///     ("sunset", h(20)),
    ///   ].into_iter().collect::<HashMap<String, u32>>();
    /// )
    /// ```
    pub fn define_variables(&mut self, variables: HashMap<String, u32>) {
        self.variables = variables;
    }

    /// Checks if a value is in a time-range
    /// # Examples
    /// ```
    /// let parser = TimeRangeParser::new();
    ///
    /// assert!(parser.matches_time_range(&(h(10), h(20)), h(12)));
    /// assert!(parser.matches_time_range(&(h(10), h(20)), h(10)));
    /// assert!(parser.matches_time_range(&(h(12), h(6)), h(20)));
    /// assert!(!parser.matches_time_range(&(h(12), h(6)), h(8)));
    /// ```
    pub fn matches_time_range(&self, range: &TimeRange, value: u32) -> bool {
        if range.0 < range.1 {
            value >= range.0 && value < range.1
        } else {
            value >= range.0 || value < range.1
        }
    }

    /// Converts a 24h timestamp to minutes
    /// # Examples
    /// ```
    /// let parser = TimeRangeParser::new();
    ///
    /// assert_eq!(parser.extract_minutes("12:23", 24), Some(743));
    /// assert_eq!(parser.extract_minutes("12", 24), Some(720));
    /// assert_eq!(parser.extract_minutes("0:00", 24), Some(0));
    /// ```
    fn extract_minutes(&self, str: &str, max_hours: u32) -> Option<u32> {
        let parts = str.split(":").collect::<Vec<&str>>();

        if parts.len() > 0 && parts.len() < 3 {
            let hours = parts[0].parse::<u32>().ok()?;
            let minutes = if parts.len() > 1 {
                parts[1].parse::<u32>().ok()?
            } else {
                0
            };

            if minutes > 59 || hours > max_hours {
                None
            } else {
                Some(h(hours) + minutes)
            }
        } else {
            None
        }
    }

    /// Extracts a time-segment from a string, uses variables if defined
    /// # Examples
    /// ```
    /// let parser = TimeRangeParser::new();
    ///
    /// assert_eq!(parser.extract_time_segment("12:23h"), Some(743));
    /// assert_eq!(parser.extract_time_segment("12h"), Some(720));
    /// assert_eq!(parser.extract_time_segment("0:00h"), Some(0));
    /// assert_eq!(parser.extract_time_segment("5AM"), Some(300));
    /// ```
    fn extract_time_segment(&self, str: &str) -> Option<u32> {
        if let Some(parsed) = self.regex_24h.captures(str) {
            return self.extract_minutes(&parsed["value"], 24);
        } else if let Some(parsed) = self.regex_12h.captures(str) {
            let minutes = self.extract_minutes(&parsed["value"], 12)?;
            let format = &parsed["format"];

            return if format == "AM" && minutes >= h(12) {
                Some(minutes - h(12))
            } else if format == "PM" && minutes < h(12) {
                Some(minutes + h(12))
            } else {
                Some(minutes)
            };
        } else if let Some(value) = self.variables.get(str) {
            return Some(*value);
        }

        None
    }

    /// Extracts a time-range from a string
    /// # Examples
    /// ```
    /// let parser = TimeRangeParser::new();
    ///
    /// assert_eq!(parser.extract_time_range("Test"), None);
    /// assert_eq!(parser.extract_time_range("Test (10h-20h)"), Some((h(10), h(20))));
    /// assert_eq!(parser.extract_time_range("Test (12:23h-20h)"), Some((h(12) + 23, h(20))));
    /// assert_eq!(parser.extract_time_range("Test (12:23h-20:59h)"), Some((h(12) + 23, h(20) + 59)));
    /// assert_eq!(parser.extract_time_range("Test (5AM-6PM)"), Some((h(5), h(18))));
    /// assert_eq!(parser.extract_time_range("Test (12AM-12PM)"), Some((h(0), h(12))));
    /// assert_eq!(parser.extract_time_range("Test (12:59AM-12:59PM)"), Some((h(0) + 59, h(12) + 59)));
    /// ```
    pub fn extract_time_range(&self, str: &str) -> Option<TimeRange> {
        let parsed = self.regex_range.captures(str)?;

        Some((
            self.extract_time_segment(&parsed["from"])?,
            self.extract_time_segment(&parsed["to"])?,
        ))
    }

    /// Extracts multiple time-ranges from a string
    /// # Examples
    /// ```
    /// let parser = TimeRangeParser::new();
    ///
    /// assert_eq!(parser.extract_time_ranges("Test"), vec![]);
    /// assert_eq!(parser.extract_time_ranges("Test (10h-20h)"), vec![(h(10), h(20))]);
    /// assert_eq!(parser.extract_time_ranges("Test (10h-20h, 12h-14h)"), vec![(h(10), h(20)), (h(12), h(14))]);
    /// assert_eq!(parser.extract_time_ranges("Test (10h-20h, 12h-14h, 16h-18h)"), vec![(h(10), h(20)), (h(12), h(14)), (h(16), h(18))]);
    /// ```
    pub fn extract_time_ranges(&self, str: &str) -> Vec<TimeRange> {
        let Some(parsed) = self.regex_ranges.captures(str) else {
            return vec![];
        };

        parsed["values"]
            .split(",")
            .into_iter()
            .filter_map(|value| self.extract_time_range(value.trim()))
            .collect::<Vec<TimeRange>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_time_range() {
        let parser = TimeRangeParser::new();
        let etr = |v: &str| parser.extract_time_range(v);

        assert_eq!(etr("10h-20h"), Some((h(10), h(20))));
        assert_eq!(etr("12:23h-20:59h"), Some((h(12) + 23, h(20) + 59)));
        assert_eq!(etr("0:01h-0:00h"), Some((1, 0)));
        assert_eq!(etr("0:00h-0:00h"), Some((0, 0)));
        assert_eq!(etr("0:1h-0:0h"), None);
        assert_eq!(etr("10h-20:60h"), None);

        assert_eq!(etr("6AM-6PM"), Some((h(6), h(18))));
        assert_eq!(etr("12AM-12PM"), Some((h(0), h(12))));
        assert_eq!(etr("12:59AM-12:59PM"), Some((h(0) + 59, h(12) + 59)));
        assert_eq!(etr("2:30PM-1:55PM"), Some((h(14) + 30, h(13) + 55)));
        assert_eq!(etr("13PM-6PM"), None);
        assert_eq!(etr("3AM-16:15h"), Some((h(3), h(16) + 15)));
    }

    #[test]
    fn test_matches_time_range() {
        let parser = TimeRangeParser::new();
        let mtr = |r: &TimeRange, v: u32| parser.matches_time_range(&r, v);

        assert!(mtr(&(h(10), h(20)), h(12)));
        assert!(mtr(&(h(10), h(20)), h(19)));
        assert!(!mtr(&(h(10), h(20)), h(20)));
        assert!(mtr(&(h(12), h(6)), h(20)));
        assert!(!mtr(&(h(12), h(6)), h(8)));
        assert!(!mtr(&(h(12), h(6)), h(8)));
        assert!(mtr(&(h(12), h(6)), h(12)));
        assert!(mtr(&(h(12), h(6)), h(18)));
        assert!(!mtr(&(h(12), h(6)), h(6)));
        assert!(mtr(&(h(12), h(6)), h(4)));
        assert!(mtr(&(h(20), h(12)), h(21)));
        assert!(mtr(&(h(20), h(12)), h(10)));
        assert!(!mtr(&(h(20), h(12)), h(13)));
    }

    #[test]
    fn test_time_ranges() {
        let parser = TimeRangeParser::new();
        let etrs = |v: &str| parser.extract_time_ranges(v);

        assert_eq!(etrs("Test"), vec![]);
        assert_eq!(etrs("Test (10h-20h)"), vec![(h(10), h(20))]);

        assert_eq!(
            etrs("Test (10h-20h, 12h-14h)"),
            vec![(h(10), h(20)), (h(12), h(14))]
        );

        assert_eq!(
            etrs("Test (10h-20h, 12h-14h, 16h-18h)"),
            vec![(h(10), h(20)), (h(12), h(14)), (h(16), h(18))]
        );
    }

    #[test]
    fn test_time_range_with_variables() {
        let mut parser = TimeRangeParser::new();

        parser.define_variables(HashMap::from([
            ("sunrise".to_string(), h(6)),
            ("sunset".to_string(), h(20)),
        ]));

        let etr = |v: &str| parser.extract_time_range(v);

        assert_eq!(etr("sunrise-sunset"), Some((h(6), h(20))));
        assert_eq!(etr("sunrise-20h"), Some((h(6), h(20))));
        assert_eq!(etr("18:23h-sunset"), Some((h(18) + 23, h(20))));
        assert_eq!(etr("18:23h-15h"), Some((h(18) + 23, h(15))));
    }
}
