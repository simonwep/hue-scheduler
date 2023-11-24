use regex::Regex;
use std::collections::HashMap;

pub struct TimeRangeParser {
    regex_range: Regex,
    regex_24h_time: Regex,
    variables: HashMap<String, u32>,
}

impl TimeRangeParser {
    pub fn new() -> TimeRangeParser {
        TimeRangeParser {
            regex_range: Regex::new(r"\((?<from>.*?)-(?<to>.*?)\)").unwrap(),
            regex_24h_time: Regex::new(r"^(?<value>\d{1,2}(:\d{2})?)h$").unwrap(),
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
    ///     ("sunrise", 6 * 60),
    ///     ("sunset", 20 * 60),
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
    /// assert!(parser.matches_time_range(&(10 * 60, 20 * 60), 12 * 60));
    /// assert!(parser.matches_time_range(&(10 * 60, 20 * 60), 10 * 60));
    /// assert!(parser.matches_time_range(&(12 * 60, 6 * 60), 20 * 60));
    /// assert!(!parser.matches_time_range(&(12 * 60, 6 * 60), 8 * 60));
    /// ```
    pub fn matches_time_range(&self, range: &(u32, u32), value: u32) -> bool {
        if range.0 < range.1 {
            value >= range.0 && value <= range.1
        } else {
            value >= range.0 || value <= range.1
        }
    }

    /// Converts a 24h timestamp to minutes
    /// # Examples
    /// ```
    /// let parser = TimeRangeParser::new();
    ///
    /// assert_eq!(parser.extract_minutes("12:23"), Some(743));
    /// assert_eq!(parser.extract_minutes("12"), Some(720));
    /// assert_eq!(parser.extract_minutes("0:00"), Some(0));
    /// ```
    pub fn extract_minutes(&self, str: &str) -> Option<u32> {
        let parts = str.split(":").collect::<Vec<&str>>();

        if parts.len() > 0 && parts.len() < 3 {
            let hours = parts[0].parse::<u32>().ok()?;
            let minutes = if parts.len() > 1 {
                parts[1].parse::<u32>().ok()?
            } else {
                0
            };

            if minutes > 59 || hours > 24 {
                None
            } else {
                Some(hours * 60 + minutes)
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
    /// ```
    fn extract_time_segment(&self, str: &str) -> Option<u32> {
        if let Some(parsed) = self.regex_24h_time.captures(str) {
            return self.extract_minutes(&parsed["value"]);
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
    /// assert_eq!(parser.extract_time_range("Test (10h-20h)"), Some((10 * 60, 20 * 60)));
    /// assert_eq!(parser.extract_time_range("Test (12:23h-20h)"), Some((12 * 60 + 23, 20 * 60)));
    /// assert_eq!(parser.extract_time_range("Test (12:23h-20:59h)"), Some((12 * 60 + 23, 20 * 60 + 59)));
    /// ```
    pub fn extract_time_range(&self, str: &str) -> Option<(u32, u32)> {
        let parsed = self.regex_range.captures(str)?;

        Some((
            self.extract_time_segment(&parsed["from"])?,
            self.extract_time_segment(&parsed["to"])?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_time_range() {
        let parser = TimeRangeParser::new();

        let tests = [
            ("Test (10h-20h)", Some((10 * 60, 20 * 60))),
            ("Test (12:23h-20h)", Some((12 * 60 + 23, 20 * 60))),
            ("Test (12:23h-20:59h)", Some((12 * 60 + 23, 20 * 60 + 59))),
            ("Test (0:01h-0:00h)", Some((1, 0))),
            ("Test (0:00h-0:00h)", Some((0, 0))),
            ("Test (0:1h-0:0h)", None),
            ("Test (10h-20:60h)", None),
            ("Test (10h-25h)", None),
            ("Test (10h-20h", None),
            ("Test 10h-20h)", None),
        ];

        for test in tests.iter() {
            assert_eq!(parser.extract_time_range(test.0), test.1);
        }
    }

    #[test]
    fn test_matches_time_range() {
        let parser = TimeRangeParser::new();

        assert!(parser.matches_time_range(&(10 * 60, 20 * 60), 12 * 60));
        assert!(parser.matches_time_range(&(10 * 60, 20 * 60), 10 * 60));
        assert!(parser.matches_time_range(&(12 * 60, 6 * 60), 20 * 60));
        assert!(!parser.matches_time_range(&(12 * 60, 6 * 60), 8 * 60));
        assert!(parser.matches_time_range(&(20 * 60, 12 * 60), 21 * 60));
        assert!(parser.matches_time_range(&(20 * 60, 12 * 60), 10 * 60));
        assert!(!parser.matches_time_range(&(20 * 60, 12 * 60), 13 * 60));
    }

    #[test]
    fn test_time_range_with_variables() {
        let mut parser = TimeRangeParser::new();

        parser.define_variables(HashMap::from([
            ("sunrise".to_string(), 6 * 60),
            ("sunset".to_string(), 20 * 60),
        ]));

        let tests = [
            ("Test (sunrise-sunset)", Some((6 * 60, 20 * 60))),
            ("Test (sunrise-20h)", Some((6 * 60, 20 * 60))),
            ("Test (18:23h-sunset)", Some((18 * 60 + 23, 20 * 60))),
            ("Test (18:23h-15h)", Some((18 * 60 + 23, 15 * 60))),
        ];

        for test in tests.iter() {
            assert_eq!(parser.extract_time_range(test.0), test.1);
        }
    }
}
