use crate::utils::get_scheduled_scenes;
use huelib2::resource::group::StateModifier;
use huelib2::resource::{Light, Scene};
use huelib2::Bridge;
use std::collections::HashMap;

mod config;
mod utils;

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

        // Process scenes
        for scheduled_scene in get_scheduled_scenes(&changed_scenes).iter() {
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
