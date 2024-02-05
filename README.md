<br/>

<h3 align="center">
    ⏱️ Better Philips Hue Automation 🪄
</h3>

<p align="center">
  <a href="https://github.com/simonwep/hue-scheduler/actions?query=workflow%3AMain"><img
     alt="CI Status"
     src="https://github.com/simonwep/hue-scheduler/workflows/Main/badge.svg"/></a>
</p>

### Summary

Philips hue automations have one large disadvantage - if the light isn't reachable when the automation is supposed to run, it never does.
This app checks every time a light is reachable again, if it corresponds to a scene and sets this scene.
Open an [issue](https://github.com/simonwep/hue-scheduler/issues) if you find anything missing :)

What's even better is, that you can keep using your physical switches without replacing them!

### Installation

Currently, the only way to install this app is to clone this repository and build it on the platform of your choice.
[Pre-compiled binaries](https://github.com/simonwep/hue-scheduler/issues/6) are planned, but not prioritized since platforms may vary heavily.

1. Download the [rust](https://www.rust-lang.org/tools/install) compiler.
2. Clone this repo via `git clone https://github.com/simonwep/hue-scheduler`.
3. Copy `.env.example` to `.env` and fill out missing values.
4. Run `cargo build --release`.
5. You can now execute `/target/release/hue-scheduler` as you want, a [service](https://linuxhandbook.com/create-systemd-services/) is recommended.
   Make sure to specify the working directory where your `.env` lies.

> [!TIP]
> After installation and setup (e.g. the app is running) nothing needs to be done anymore.  
> Anything else is configured in your Philips Hue app!

### Usage

When which scene should be turned on is solely specified by the name of your scenes.
The format is as follows: `{name of your scene} ({timestamp}-{timestamp}, ...)`, where `{timestamp}` can be:

- In the 24h format: `12h`, `13:45h`, `0h`, `9:20h`
- In the 12h format: `3AM`, `8PM`, `11PM`
- A variable: `sunrise`, `sunset` (depending on `HOME_LATITUDE` and `HOME_LONGITUDE` in your `.env`)

#### Examples

Example scene names with time-frames:

- **Natural light (8AM-10:30h, 17h-sunset)** _- The "Natural light" scene should be turned on from 8:00 AM to 10:30 AM and from 5:00 PM until sunset._
- **Night light (sunset-11PM)** _- The "Night light" scene should be turned on from sunset until 11:00 PM._
- **Wake up (sunrise-8:30h)** _- The "Wake up" scene should be turned on from sunrise until 8:30 AM._
- **Work (8:30h-17h)** _- The "Work" scene should be turned on from 8:30 AM until 5:00 PM._
- **Sleep (11PM-8AM)** _- The "Sleep" scene should be turned on from 11:00 PM until 8:00 AM._

#### Working with "always-on" lights

Some lights may always be reachable and should be turned on when a scene is activated due to another light that is controlled by a physical switch.
To mark a light to be turned on/off as well whenever the corresponding scene is activated/deactivated, prepend a `(att)` for "attached" to the lights name.

Now, if you flip the physical switch and the light is turned off the lights that are always "on" (connected to a power source) will be turned off as well.
Since it takes some time for the hue bridge to recognize no longer reachable lights this may take up to a minute.
Still better than doing it manually ;)

### Screenshots

This is how it will usually look like in the app.
A scene will be picked up and turned on if all corresponding lights _became_ (e.g. were not) reachable again.
The timeframe for that (and much more) can be configured in the `.env` file.

<p float="left" align="center">
  <img src="https://github.com/simonwep/hue-scheduler/assets/30767528/d0d84e46-4dcf-4846-9063-f5052ab69b98" width="250" alt="Screenshot"/>
  <img src="https://github.com/simonwep/hue-scheduler/assets/30767528/40c40248-09bf-4db9-b7eb-44ae78de194d" width="250" alt="Screenshot"/>
  <img src="https://github.com/simonwep/hue-scheduler/assets/30767528/788572ea-ce6a-4976-aee1-a49393e9b24d" width="250" alt="Screenshot"/>
</p>
