use crate::time_range_parser::TimeRangeParser;
use chrono::{Local, Timelike};
use huelib2::resource::group::StateModifier;
use huelib2::resource::{Light, Scene};
use huelib2::Bridge;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Instant;
use sun_times::sun_times;

mod config;
mod time_range_parser;

#[derive(Clone, PartialEq, Debug)]
struct ScheduledScene {
    pub scene_id: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, PartialEq, Debug)]
struct StateChange {
    pub timestamp: Option<Instant>,
    pub reachable: bool,
}

fn main() {
    let mut light_states = HashMap::<String, StateChange>::new();
    let conf = config::load_config();
    let bridge = Bridge::new(conf.bridge_ip, conf.bridge_username);

    loop {
        std::thread::sleep(conf.ping_interval);

        let Ok(all_lights) = bridge.get_all_lights() else {
            eprintln!("Failed to retrieve lights");
            continue;
        };

        // Check for light changes
        let changed_lights = all_lights
            .into_iter()
            .filter(|light| {
                light_states
                    .get(&light.id)
                    .map(|last_reachable| last_reachable.reachable != light.state.reachable)
                    .unwrap_or(true)
            })
            .collect::<Vec<Light>>();

        if changed_lights.is_empty() {
            continue;
        }

        if light_states.is_empty() {
            for light in changed_lights.iter() {
                light_states.insert(
                    light.id.clone(),
                    StateChange {
                        timestamp: None,
                        reachable: light.state.reachable,
                    },
                );
            }

            println!("Initialized reachable lights.");
            continue;
        }

        // Update reachable lights
        for light in changed_lights.iter() {
            if let Some(last_reachable) = light_states.get(&light.id) {
                if last_reachable.reachable && !light.state.reachable {
                    println!("Light \"{}\" is not reachable anymore", light.name)
                } else {
                    println!("Light \"{}\" is reachable again", light.name);
                };
            };

            light_states.insert(
                light.id.clone(),
                StateChange {
                    timestamp: Some(Instant::now()),
                    reachable: light.state.reachable,
                },
            );
        }

        // Check for scene changes, this is done by:
        // 1. Extract all reachable lights that have been reachable for less than the reachability window
        // 2. Extract all scenes that contain all the lights from 1.
        let light_trigger_ids = light_states
            .iter()
            .filter(|(_, state)| {
                state.reachable
                    && state
                        .timestamp
                        .map(|timestamp| timestamp.elapsed() < conf.reachability_window)
                        .unwrap_or(false)
            })
            .map(|(light_id, _)| light_id)
            .collect::<Vec<&String>>();

        // Extract scenes from which all lights are reachable
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
                                .all(|light_id| light_trigger_ids.contains(&light_id))
                        })
                        .unwrap_or(false)
                })
                .collect::<Vec<Scene>>()
        }) else {
            eprintln!("Failed to retrieve scenes");
            continue;
        };

        // Reset timestamp to prevent scenes to be set multiple times
        for changed_scene in changed_scenes.iter() {
            if let Some(lights) = &changed_scene.lights {
                for light_id in lights.clone() {
                    light_states.insert(
                        light_id,
                        StateChange {
                            timestamp: None,
                            reachable: true,
                        },
                    );
                }
            }
        }

        let Some((sunrise, sunset)) = get_sunrise_sunset(conf.home_latitude, conf.home_longitude)
        else {
            eprintln!("Failed to retrieve sunrise/sunset");
            continue;
        };

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

fn get_sunrise_sunset(latitude: f64, longitude: f64) -> Option<(u32, u32)> {
    let (sunrise, sunset) = sun_times(Local::now().date_naive(), latitude, longitude, 0f64)?;

    Some((
        sunrise.hour() * 60 + sunrise.minute(),
        sunset.hour() * 60 + sunset.minute(),
    ))
}
