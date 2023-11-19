use chrono::{Local, Timelike};
use huelib2::resource::scene::Scene;
use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Clone, PartialEq, Debug)]
pub struct ScheduledScene {
    pub scene_id: String,
    pub start: u32,
    pub end: u32,
}

pub fn extract_minutes(str: &String) -> Result<Option<u32>, ()> {
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
pub fn extract_time_range(str: &String) -> Option<(u32, u32)> {
    let time = Regex::new(r"\((?<start>\d{1,2}(:\d{2})?)h-(?<end>\d{1,2}(:\d{2})?)h\)$").unwrap();
    let parsed = time.captures(str.as_str())?;

    Some((
        extract_minutes(&parsed["start"].to_string()).ok()??,
        extract_minutes(&parsed["end"].to_string()).ok()??,
    ))
}

/// Returns all scheduled scenes that are active right now
pub fn get_scheduled_scenes(scenes: &Vec<Scene>) -> Vec<ScheduledScene> {
    let mut scheduled_scenes = HashMap::<u64, ScheduledScene>::new();
    let now = Local::now().hour() * 60 + Local::now().minute();

    // Group scenes by their lights
    for scene in scenes {
        let Some(time_range) = extract_time_range(&scene.name) else {
            continue;
        };

        // Out of range
        if now < time_range.0 || now > time_range.1 {
            continue;
        }

        let Some(lights) = &scene.lights else {
            continue;
        };

        let mut sorted_lights = lights.clone();
        sorted_lights.sort();

        let mut hash = DefaultHasher::new();
        sorted_lights.hash(&mut hash);

        let scene_id = hash.finish();

        // Check if scene is closer to now than this one
        if let Some(last_scene) = scheduled_scenes.get(&scene_id) {
            if last_scene.start > time_range.0 {
                continue;
            }
        }

        scheduled_scenes.insert(
            scene_id,
            ScheduledScene {
                scene_id: scene.id.clone(),
                start: time_range.0,
                end: time_range.1,
            },
        );
    }

    scheduled_scenes
        .values()
        .cloned()
        .collect::<Vec<ScheduledScene>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_time_range() {
        // Valid formats
        assert_eq!(
            extract_time_range(&"Test (10h-20h)".to_string()),
            Some((10 * 60, 20 * 60))
        );
        assert_eq!(
            extract_time_range(&"Test (12:23h-20h)".to_string()),
            Some((12 * 60 + 23, 20 * 60))
        );
        assert_eq!(
            extract_time_range(&"Test (12:23h-20:59h)".to_string()),
            Some((12 * 60 + 23, 20 * 60 + 59))
        );
        assert_eq!(
            extract_time_range(&"Test (0:01h-0:00h)".to_string()),
            Some((1, 0))
        );
        assert_eq!(
            extract_time_range(&"Test (0:00h-0:00h)".to_string()),
            Some((0, 0))
        );

        // Invalid formats
        assert_eq!(extract_time_range(&"Test (0:1h-0:0h)".to_string()), None);
        assert_eq!(extract_time_range(&"Test (10h-20:60h)".to_string()), None);
        assert_eq!(extract_time_range(&"Test (10h-25h)".to_string()), None);
        assert_eq!(extract_time_range(&"Test (10h-20h".to_string()), None);
        assert_eq!(extract_time_range(&"Test 10h-20h)".to_string()), None);
    }
}
