use crate::time_range_parser::TimeRangeParser;
use chrono::{DateTime, Local, Utc};
use huelib2::resource::group::StateModifier;
use huelib2::resource::{Light, Scene};
use huelib2::Bridge;
use std::collections::HashMap;
use std::time::Instant;

mod config;
mod time_range_parser;
mod utils;

#[derive(Clone, PartialEq, Debug)]
struct StateChange {
    pub timestamp: Option<Instant>,
    pub reachable: bool,
}

fn main() {
    let mut light_states = HashMap::<String, StateChange>::new();
    let conf = config::load_config();
    let bridge = Bridge::new(conf.bridge_ip.clone(), &conf.bridge_username);

    println!(
        "Starting hue-scheduler at {}",
        DateTime::<Utc>::from(Local::now())
            .with_timezone(&conf.home_timezone)
            .format("%Y-%m-%d %H:%M:%S %Z")
    );

    loop {
        std::thread::sleep(conf.ping_interval);

        let all_lights = match bridge.get_all_lights() {
            Ok(result) => result,
            Err(error) => {
                eprintln!("Failed to retrieve lights: {:?}", error);
                continue;
            }
        };

        // Check for light changes
        let changed_lights = all_lights
            .iter()
            .filter(|light| {
                !utils::is_attached_light(light)
                    && light_states
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
            .filter(|light| utils::is_attached_light(light))
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

        let Some((sunrise, sunset)) =
            utils::get_sunrise_sunset(conf.home_latitude, conf.home_longitude)
        else {
            eprintln!("Failed to retrieve sunrise/sunset");
            continue;
        };

        let mut parser = TimeRangeParser::new();
        parser.define_variables(HashMap::from([
            ("sunrise".to_string(), sunrise),
            ("sunset".to_string(), sunset),
        ]));

        // Turn on currently scheduled scenes
        for scheduled_scene in utils::get_scheduled_scenes(&conf, &parser, &changed_scenes).iter() {
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

        // Turn off all groups where all lights that are not marked as attached are no longer reachable.
        for group in all_groups.iter() {
            let some_lights_on = group.lights.iter().any(|light_id| {
                all_lights
                    .iter()
                    .find(|light| light.id == *light_id)
                    .map(|light| light.state.on.unwrap_or(false))
                    .unwrap_or(false)
            });

            let all_non_attached_turned_off = group.lights.iter().all(|light_id| {
                ignored_light_ids.contains(&light_id)
                    || (light_states
                        .get(light_id)
                        .map(|state| !state.reachable)
                        .unwrap_or(false))
            });

            if some_lights_on && all_non_attached_turned_off {
                println!(
                    "All non-atteched lights are unreachable, turning off group: {}",
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
