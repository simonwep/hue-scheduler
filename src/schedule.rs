use regex::Regex;
use std::num::ParseIntError;

#[derive(Clone, PartialEq, Debug)]
pub struct ScheduledScene {
    pub scene_id: String,
    pub start: u32,
    pub end: u32,
}

impl ScheduledScene {
    pub fn new(scene_id: &str, start: u32, end: u32) -> ScheduledScene {
        ScheduledScene {
            scene_id: scene_id.to_string(),
            start,
            end,
        }
    }
}

pub fn extract_minutes(str: &String) -> Result<Option<u32>, ParseIntError> {
    let parts = str.split(":").collect::<Vec<&str>>();

    if parts.len() > 0 && parts.len() < 3 {
        Ok(Some(
            parts[0].parse::<u32>()?
                + if parts.len() > 1 {
                    parts[1].parse::<u32>()?
                } else {
                    0
                },
        ))
    } else {
        Ok(None)
    }
}

/// Extracts a time-range from a string
pub fn extract_time_range(str: &String) -> Option<(u32, u32)> {
    let time = Regex::new(r"\((?<start>\d{1,2}(:\d{2})?)h-(?<end>\d{1,2}(:\d{2})?)h\)$").unwrap();
    let parsed = time.captures(str.as_str())?;

    Some((
        extract_minutes(&parsed["start"].to_string()).ok()??,
        extract_minutes(&parsed["end"].to_string()).ok()??,
    ))
}

/// Linearizes all scenes in case of overlapping time ranges
/// Returns a sorted, linearized list of schedules without overlaps
pub fn linearize_schedules(list: Vec<ScheduledScene>) -> Vec<ScheduledScene> {
    let mut sorted_list = list.clone();
    sorted_list.sort_by(|a, b| a.start.cmp(&b.start));

    let mut stack = Vec::<ScheduledScene>::new();
    let mut schedules = Vec::<ScheduledScene>::new();

    // Build up stack
    for current_schedule in sorted_list {
        let Some(last_schedule) = stack.last_mut() else {
            stack.push(current_schedule.clone());
            continue;
        };

        // Last schedule does not overlap
        if current_schedule.start >= last_schedule.end {
            schedules.push(stack.pop().unwrap());
            stack.push(current_schedule);
            continue;
        }

        // Schedule would never be active
        if current_schedule.start <= last_schedule.start {
            stack.push(current_schedule);
            continue;
        }

        // Check if schedule can be merged with previous one
        if last_schedule.scene_id == current_schedule.scene_id {
            last_schedule.end = current_schedule.start;
        } else {
            schedules.push(ScheduledScene {
                scene_id: last_schedule.scene_id.clone(),
                start: last_schedule.start,
                end: current_schedule.start,
            });
        }

        stack.push(current_schedule);
    }

    // Unwind stack
    while let Some(schedule) = stack.pop() {
        let Some(last_schedule) = schedules.last_mut() else {
            schedules.push(schedule);
            continue;
        };

        if schedule.start >= last_schedule.end {
            if last_schedule.scene_id == schedule.scene_id {
                last_schedule.end = schedule.end;
            } else {
                schedules.push(schedule);
            }
        } else if schedule.end > last_schedule.end {
            if last_schedule.scene_id == schedule.scene_id {
                last_schedule.end = schedule.end;
            } else {
                let end = last_schedule.end;
                schedules.push(ScheduledScene {
                    scene_id: schedule.scene_id.clone(),
                    start: end,
                    end: schedule.end,
                });
            }
        }
    }

    schedules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linearize_schedules_no_change() {
        assert_eq!(
            linearize_schedules(vec![ScheduledScene::new("1", 0, 10)]),
            vec![ScheduledScene::new("1", 0, 10)]
        );

        assert_eq!(
            linearize_schedules(vec![
                ScheduledScene::new("1", 0, 10),
                ScheduledScene::new("2", 50, 100),
                ScheduledScene::new("3", 100, 200),
            ]),
            vec![
                ScheduledScene::new("1", 0, 10),
                ScheduledScene::new("2", 50, 100),
                ScheduledScene::new("3", 100, 200),
            ]
        );
    }

    #[test]
    fn test_linearize_schedules_overlapping() {
        assert_eq!(
            linearize_schedules(vec![
                ScheduledScene::new("1", 0, 100),
                ScheduledScene::new("2", 25, 75),
            ]),
            vec![
                ScheduledScene::new("1", 0, 25),
                ScheduledScene::new("2", 25, 75),
                ScheduledScene::new("1", 75, 100),
            ]
        );
    }

    #[test]
    fn test_linearize_schedules_enclosed() {
        assert_eq!(
            linearize_schedules(vec![
                ScheduledScene::new("1", 0, 100),
                ScheduledScene::new("2", 25, 100),
                ScheduledScene::new("2", 0, 25),
            ]),
            vec![ScheduledScene::new("2", 0, 100)]
        );
    }

    #[test]
    fn test_linearize_schedules_overflow() {
        assert_eq!(
            linearize_schedules(vec![
                ScheduledScene::new("1", 0, 100),
                ScheduledScene::new("2", 50, 100),
                ScheduledScene::new("3", 70, 120),
                ScheduledScene::new("2", 75, 100),
            ]),
            vec![
                ScheduledScene::new("1", 0, 50),
                ScheduledScene::new("2", 50, 70),
                ScheduledScene::new("3", 70, 75),
                ScheduledScene::new("2", 75, 100),
                ScheduledScene::new("3", 100, 120),
            ]
        );
    }
}
