use crate::config::Config;
use crate::time_range_parser::TimeRangeParser;
use chrono::{DateTime, Local, Timelike, Utc};
use huelib2::resource::{Light, Scene};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Clone, PartialEq, Debug)]
pub struct ScheduledScene {
    pub scene_id: String,
    pub start: u32,
    pub end: u32,
}

/// Returns all scheduled scenes that are active right now
pub fn get_scheduled_scenes(
    conf: &Config,
    parser: &TimeRangeParser,
    scenes: &Vec<Scene>,
) -> Vec<ScheduledScene> {
    let mut scheduled_scenes = HashMap::<u64, ScheduledScene>::new();
    let date_time = DateTime::<Utc>::from(Local::now()).with_timezone(&conf.home_timezone);
    let now = date_time.hour() * 60 + date_time.minute();

    // Group scenes by their lights
    for scene in scenes {
        let time_ranges = parser.extract_time_ranges(&scene.name);

        let Some(time_range) = time_ranges
            .iter()
            .find(|range| parser.matches_time_range(range, now))
        else {
            continue;
        };

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

pub fn get_sunrise_sunset(latitude: f64, longitude: f64) -> Option<(u32, u32)> {
    let (sunrise, sunset) =
        sun_times::sun_times(Local::now().date_naive(), latitude, longitude, 0f64)?;

    Some((
        sunrise.hour() * 60 + sunrise.minute(),
        sunset.hour() * 60 + sunset.minute(),
    ))
}

pub fn is_attached_light(light: &Light) -> bool {
    light.name.ends_with("(att)")
}
