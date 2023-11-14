use std::cmp::Ordering;
use std::num::ParseIntError;
use regex::Regex;

#[derive(Debug)]
pub struct DayTime {
    pub hour: u32,
    pub minute: u32,
}

#[derive(Debug)]
pub struct TimeRange {
    pub start: DayTime,
    pub end: DayTime,
}


impl Eq for DayTime {}

impl PartialEq<Self> for DayTime {
    fn eq(&self, other: &Self) -> bool {
        self.hour == other.hour && self.minute == other.minute
    }
}

impl PartialOrd<Self> for DayTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.minute.partial_cmp(&other.minute).and(self.hour.partial_cmp(&other.hour))
    }
}

impl Ord for DayTime {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.hour == other.hour {
            self.minute.cmp(&other.minute)
        } else {
            self.hour.cmp(&other.hour)
        }
    }
}

impl DayTime {
    pub fn is_between(&self, range: &TimeRange) -> bool {
        self >= &range.start && self <= &range.end
    }
}

pub fn extract_day_time(str: &String) -> Result<Option<DayTime>, ParseIntError> {
    let parts = str.split(":").collect::<Vec<&str>>();

    if parts.len() > 0 && parts.len() < 3 {
        Ok(Some(DayTime {
            hour: parts[0].parse::<u32>()?,
            minute: if parts.len() > 1 { parts[1].parse::<u32>()? } else { 0 },
        }))
    } else {
        Ok(None)
    }
}

pub fn extract_time_range(str: &String) -> Option<TimeRange> {
    let time = Regex::new(r"\((?<start>\d{1,2}(:\d{2})?)h-(?<end>\d{1,2}(:\d{2})?)h\)$").unwrap();
    let parsed = time.captures(str.as_str())?;

    Some(TimeRange {
        start: extract_day_time(&parsed["start"].to_string()).ok()??,
        end: extract_day_time(&parsed["end"].to_string()).ok()??,
    })
}