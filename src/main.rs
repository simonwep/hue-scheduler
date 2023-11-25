use crate::time_range_parser::TimeRangeParser;
use chrono::{Datelike, Local, Timelike};
use huelib2::resource::group::StateModifier;
use huelib2::resource::{Light, Scene};
use huelib2::Bridge;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

mod config;
mod time_range_parser;

#[derive(Clone, PartialEq, Debug)]
pub struct ScheduledScene {
    pub scene_id: String,
    pub start: u32,
    pub end: u32,
}

fn main() {
    let mut reachable = HashMap::<String, bool>::new();
    let configuration = config::load_config();
    let bridge = Bridge::new(configuration.bridge_ip, configuration.bridge_username);

    loop {
        std::thread::sleep(configuration.interval);

        let Ok(all_lights) = bridge.get_all_lights() else {
            eprintln!("Failed to retrieve lights");
            continue;
        };

        // Check for light changes
        let changed_lights = all_lights
            .into_iter()
            .filter(|light| {
                let Some(last_reachable) = reachable.get(&light.id) else {
                    return true;
                };
                *last_reachable != light.state.reachable
            })
            .collect::<Vec<Light>>();

        if changed_lights.is_empty() {
            continue;
        }

        if reachable.is_empty() {
            for light in changed_lights.iter() {
                reachable.insert(light.id.clone(), light.state.reachable);
            }
            continue;
        }

        // Update reachable lights
        for light in changed_lights.iter() {
            if let Some(last_reachable) = reachable.get(&light.id) {
                if *last_reachable && !light.state.reachable {
                    println!("Light \"{}\" is not reachable anymore", light.name)
                } else {
                    println!("Light \"{}\" is reachable again", light.name);
                };
            };

            reachable.insert(light.id.clone(), light.state.reachable);
        }

        // Check for scene changes
        let changed_light_ids = changed_lights
            .iter()
            .map(|light| light.id.clone())
            .collect::<Vec<String>>();

        let Ok(changed_scenes) = bridge.get_all_scenes().map(|scenes| {
            scenes
                .into_iter()
                .filter(|scene| {
                    scene
                        .lights
                        .clone()
                        .map(|light_ids| {
                            light_ids
                                .iter()
                                .all(|light_id| changed_light_ids.contains(&light_id))
                        })
                        .unwrap_or(false)
                })
                .collect::<Vec<Scene>>()
        }) else {
            eprintln!("Failed to retrieve scenes");
            continue;
        };

        let (sunrise, sunset) = get_sunrise_sunset();
        let mut parser = TimeRangeParser::new();

        parser.define_variables(HashMap::from([
            ("sunrise".to_string(), sunrise),
            ("sunset".to_string(), sunset),
        ]));

        // Process scenes
        for scheduled_scene in get_scheduled_scenes(&parser, &changed_scenes).iter() {
            if let Err(err) = bridge.set_group_state(
                &scheduled_scene.scene_id,
                &StateModifier::new().with_scene(scheduled_scene.scene_id.clone()),
            ) {
                eprintln!("Failed to set scene: {}", err);
                continue;
            }
        }
    }
}

/// Returns all scheduled scenes that are active right now
fn get_scheduled_scenes(parser: &TimeRangeParser, scenes: &Vec<Scene>) -> Vec<ScheduledScene> {
    let mut scheduled_scenes = HashMap::<u64, ScheduledScene>::new();
    let now = Local::now().hour() * 60 + Local::now().minute();

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

fn get_sunrise_sunset() -> (u32, u32) {
    let now = Local::now();
    let (sunrise, sunset) =
        sunrise::sunrise_sunset(43.6532, 79.3832, now.year(), now.month(), now.day());
    (daytime_to_minutes(sunrise), daytime_to_minutes(sunset))
}

fn daytime_to_minutes(millis: i64) -> u32 {
    (((millis / 1_000 / 60 / 60) % 24) * 60) as u32 + ((millis / 1_000 / 60) % 60) as u32
}
