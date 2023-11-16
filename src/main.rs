use crate::schedule::{extract_time_range, linearize_schedules, ScheduledScene};
use chrono::prelude::*;
use hueclient::{Bridge, IdentifiedScene};

mod config;
mod schedule;

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
    linearize_schedules(
        scenes
            .iter()
            .filter_map(|scene| {
                let (start, end) = extract_time_range(&scene.scene.name)?;
                let scene_id = scene.id.as_str();
                Some(ScheduledScene::new(scene_id, start, end))
            })
            .collect::<Vec<ScheduledScene>>(),
    )
}

/// Process schedules and turns on lights if needed
fn run(bridge: &Bridge) -> Result<(), &str> {
    //let lights = bridge.get_all_lights()?;
    let scenes = bridge
        .get_all_scenes()
        .map_err(|_| "Failed to retrieve scenes")?;
    let scheduled_scenes = get_scheduled_scenes(&scenes);
    let current_day_time = Local::now().hour() * 60 + Local::now().minute();

    // TODO: Create store with information about reachable lights and if they should be turned on or off

    for scheduled_scene in scheduled_scenes {
        let scene = scenes
            .iter()
            .find(|scene| scene.id == scheduled_scene.scene_id)
            .ok_or("Scene missing")?;

        if current_day_time > scheduled_scene.start && current_day_time < scheduled_scene.end {
            if let Some(err) = bridge.set_scene(scene.id.clone()).err() {
                println!("Failed to set scene: {} for {}", err, scene.scene.name);
            } else {
                println!("Set scene: {}", scene.scene.name);
            }
        }
    }

    Ok(())
}
