use hueclient::{Bridge, HueError, IdentifiedScene};
use chrono::prelude::*;
use crate::utils::{DayTime, TimeRange};

mod config;
mod utils;

struct ScheduledScene {
    scene_id: String,
    time_range: TimeRange,
}

fn main() {
    let conf = config::load_config();
    let bridge = Bridge::for_ip(conf.bridge_ip).with_user(conf.bridge_username);

    loop {
        std::thread::sleep(conf.interval);

        if let Some(err) = run(&bridge).err() {
            println!("Failed to run: {}", err);
        }
    }
}

fn get_scheduled_scenes(scenes: &Vec<IdentifiedScene>) -> Vec<ScheduledScene> {
    scenes
        .iter()
        .filter_map(|scene| {
            Some(ScheduledScene {
                scene_id: scene.id.clone(),
                time_range: utils::extract_time_range(&scene.scene.name)?,
            })
        }).collect::<Vec<ScheduledScene>>()
}

fn run(bridge: &Bridge) -> Result<(), HueError> {
    //let lights = bridge.get_all_lights()?;
    let scenes = bridge.get_all_scenes()?;
    let scheduled_scenes = get_scheduled_scenes(&scenes);
    let current_day_time = DayTime {
        hour: Local::now().hour(),
        minute: Local::now().minute(),
    };

    // TODO: Create store with information about reachable lights and if they should be turned on or off

    for scheduled_scene in scheduled_scenes {
        let scene = scenes.iter()
            .find(|scene| scene.id == scheduled_scene.scene_id)
            .ok_or(Err(r"Scene missing"))?;


        if current_day_time.is_between(&scheduled_scene.time_range) {
            if let Some(err) = bridge.set_scene(scene.id.clone()).err() {
                println!("Failed to set scene: {} for {}", err, scene.scene.name);
            } else {
                println!("Set scene: {}", scene.scene.name);
            }
        }
    }

    Ok(())
}
