use regex::Regex;

pub struct TimeRangeParser {
    regex_time: Regex,
}

impl TimeRangeParser {
    pub fn new() -> TimeRangeParser {
        TimeRangeParser {
            regex_time: Regex::new(r"\((?<start>\d{1,2}(:\d{2})?)h-(?<end>\d{1,2}(:\d{2})?)h\)$")
                .unwrap(),
        }
    }

    /// Checks if a value is in a time-range
    /// # Examples
    /// ```
    /// assert!(TimeRangeParser::matches_time_range(&(10 * 60, 20 * 60), 12 * 60));
    /// assert!(TimeRangeParser::matches_time_range(&(10 * 60, 20 * 60), 10 * 60));
    /// assert!(TimeRangeParser::matches_time_range(&(12 * 60, 6 * 60), 20 * 60));
    /// assert!(!TimeRangeParser::matches_time_range(&(12 * 60, 6 * 60), 8 * 60));
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
    /// assert_eq!(TimeRangeParser::extract_minutes(&"12:23".to_string()), Some(743));
    /// assert_eq!(TimeRangeParser::extract_minutes(&"12".to_string()), Some(720));
    /// assert_eq!(TimeRangeParser::extract_minutes(&"0:00".to_string()), Some(0));
    /// ```
    pub fn extract_minutes(&self, str: &String) -> Result<Option<u32>, ()> {
        let parts = str.split(":").collect::<Vec<&str>>();

        if parts.len() > 0 && parts.len() < 3 {
            let hours = parts[0].parse::<u32>().map_err(|_| ())?;
            let minutes = if parts.len() > 1 {
                parts[1].parse::<u32>().map_err(|_| ())?
            } else {
                0
            };

            if minutes > 59 || hours > 24 {
                Err(())
            } else {
                Ok(Some(hours * 60 + minutes))
            }
        } else {
            Ok(None)
        }
    }

    /// Extracts a time-range from a string
    pub fn extract_time_range(&self, str: &String) -> Option<(u32, u32)> {
        let parsed = self.regex_time.captures(str.as_str())?;

        Some((
            self.extract_minutes(&parsed["start"].to_string()).ok()??,
            self.extract_minutes(&parsed["end"].to_string()).ok()??,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_time_range() {
        let parser = TimeRangeParser::new();

        assert_eq!(
            parser.extract_time_range(&"Test (10h-20h)".to_string()),
            Some((10 * 60, 20 * 60))
        );
        assert_eq!(
            parser.extract_time_range(&"Test (12:23h-20h)".to_string()),
            Some((12 * 60 + 23, 20 * 60))
        );
        assert_eq!(
            parser.extract_time_range(&"Test (12:23h-20:59h)".to_string()),
            Some((12 * 60 + 23, 20 * 60 + 59))
        );
        assert_eq!(
            parser.extract_time_range(&"Test (0:01h-0:00h)".to_string()),
            Some((1, 0))
        );
        assert_eq!(
            parser.extract_time_range(&"Test (0:00h-0:00h)".to_string()),
            Some((0, 0))
        );

        assert_eq!(
            parser.extract_time_range(&"Test (0:1h-0:0h)".to_string()),
            None
        );
        assert_eq!(
            parser.extract_time_range(&"Test (10h-20:60h)".to_string()),
            None
        );
        assert_eq!(
            parser.extract_time_range(&"Test (10h-25h)".to_string()),
            None
        );
        assert_eq!(
            parser.extract_time_range(&"Test (10h-20h".to_string()),
            None
        );
        assert_eq!(
            parser.extract_time_range(&"Test 10h-20h)".to_string()),
            None
        );
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
}
