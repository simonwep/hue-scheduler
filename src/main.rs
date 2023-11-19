use crate::utils::get_scheduled_scenes;
use huelib2::resource::group::StateModifier;
use huelib2::Bridge;
use std::collections::HashMap;

mod config;
mod utils;

fn main() {
    let configuration = config::load_config();
    let bridge = Bridge::new(configuration.bridge_ip, configuration.bridge_username);

    let mut reachable = HashMap::<String, bool>::new();

    loop {
        std::thread::sleep(configuration.interval);

        let Ok(all_lights) = bridge.get_all_lights() else {
            eprintln!("Failed to retrieve lights");
            continue;
        };

        let Ok(all_scenes) = bridge
            .get_all_scenes()
            .map_err(|_| "Failed to retrieve scenes")
        else {
            eprintln!("Failed to retrieve scenes");
            continue;
        };

        for scheduled_scene in get_scheduled_scenes(&all_scenes).iter() {
            let Some(scene) = all_scenes
                .iter()
                .find(|scene| scene.id == scheduled_scene.scene_id)
            else {
                eprintln!("Scene not found: {}", scheduled_scene.scene_id);
                continue;
            };

            let Some(group_id) = &scene.group else {
                eprintln!("Scene has no group: {}", scene.name);
                continue;
            };

            let Some(lights) = &scene.lights else {
                eprintln!("Scene has no lights: {}", scene.name);
                continue;
            };

            let turn_on = lights.iter().all(|light_id| {
                let Some(light) = all_lights.iter().find(|light| light.id.eq(light_id)) else {
                    eprintln!("Light not found: {}", light_id);
                    return false;
                };

                let Some(last_reachable) = reachable.get(light_id) else {
                    return false;
                };

                return !*last_reachable && light.state.reachable;
            });

            if turn_on {
                let modifier = StateModifier::new().with_scene(scene.id.clone());

                if let Err(err) = bridge.set_group_state(group_id.clone(), &modifier) {
                    println!("Failed to set scene: {} for {}", err, scene.name);
                } else {
                    println!("Set scene: {}", scene.name);
                }
            }
        }

        // Update light states
        for light in all_lights.iter() {
            reachable.insert(light.id.clone(), light.state.reachable);
        }
    }
}
