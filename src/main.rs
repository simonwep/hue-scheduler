use crate::config::Config;
use crate::time_range_parser::TimeRangeParser;
use chrono::{DateTime, Local, Timelike, Utc};
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
    let bridge = Bridge::new(conf.bridge_ip.clone(), &conf.bridge_username);

    loop {
        std::thread::sleep(conf.ping_interval);

        let Ok(all_lights) = bridge.get_all_lights() else {
            eprintln!("Failed to retrieve lights");
            continue;
        };

        // Check for light changes
        let changed_lights = all_lights
            .iter()
            .filter(|light| !is_attached_light(light))
            .filter(|light| {
                light_states
                    .get(&light.id)
                    .map(|last_reachable| last_reachable.reachable != light.state.reachable)
                    .unwrap_or(true)
            })
            .collect::<Vec<&Light>>();

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

        // Collect ids of all lights that are ignored / always on / not controlled by a physical switch
        // They have the prefix "(att)" for "attached" in their name
        let ignored_light_ids = all_lights
            .iter()
            .filter(|light| is_attached_light(light))
            .map(|light| &light.id)
            .collect::<Vec<&String>>();

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

        // Extract scenes from which all lights are reachable or
        // are attached to a scene that can be triggered
        let Ok(changed_scenes) = bridge.get_all_scenes().map(|scenes| {
            scenes
                .into_iter()
                .filter(|scene| {
                    scene
                        .lights
                        .clone()
                        .map(|light_ids| {
                            light_ids.iter().all(|light_id| {
                                ignored_light_ids.contains(&light_id)
                                    || light_trigger_ids.contains(&light_id)
                            })
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
        for scheduled_scene in get_scheduled_scenes(&conf, &parser, &changed_scenes).iter() {
            // Turn on scenes
            if let Err(err) = bridge.set_group_state(
                &scheduled_scene.scene_id,
                &StateModifier::new().with_scene(scheduled_scene.scene_id.clone()),
            ) {
                eprintln!("Failed to set scene: {}", err);
                continue;
            }
        }

        // Turn of lights that are attached to scenes but reachable all the time
        let Ok(all_groups) = bridge.get_all_groups() else {
            eprintln!("Failed to retrieve groups");
            continue;
        };

        for group in all_groups.iter() {
            // Check if all lights are either attached to other lights from a scene
            // or changed their status to not reachable
            let all_non_attached_turned_off = group.lights.iter().all(|light_id| {
                ignored_light_ids.contains(&light_id)
                    || (changed_lights
                        .iter()
                        .find(|light| light.id == *light_id)
                        .is_some()
                        && light_states
                            .get(light_id)
                            .map(|state| !state.reachable)
                            .unwrap_or(false))
            });

            if all_non_attached_turned_off {
                println!(
                    "All lights are unreachable, turning off group: {}",
                    group.name
                );

                // Turn attached lights off
                if let Err(err) =
                    bridge.set_group_state(&group.id, &StateModifier::new().with_on(false))
                {
                    eprintln!("Failed to turn off attached lights: {}", err);
                    continue;
                }
            }
        }
    }
}

/// Returns all scheduled scenes that are active right now
fn get_scheduled_scenes(
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

fn get_sunrise_sunset(latitude: f64, longitude: f64) -> Option<(u32, u32)> {
    let (sunrise, sunset) = sun_times(Local::now().date_naive(), latitude, longitude, 0f64)?;

    Some((
        sunrise.hour() * 60 + sunrise.minute(),
        sunset.hour() * 60 + sunset.minute(),
    ))
}

fn is_attached_light(light: &Light) -> bool {
    light.name.ends_with("(att)")
}
