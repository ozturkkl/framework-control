- add the ability to configure profiles for the entire app and allow the ability to mark them "auto switch on a/c connect and/or disconnect" (the request came form fan profiles but maybe app profile is better?? Not sure, maybe fan profile is enough. Need to figure out a good UX for this)



Get ready to post in community forum and the reddit:

Framework Community draft

Hey everyone! A major new Framework Control update is almost ready.
[VIDEO EMBED]
This update focuses heavily on making the app more customizable, improving battery monitoring, and making fan calibration more accurate on multi-fan systems.
What’s new
- Customizable dashboard: Panels can now be dragged into a different order, switched between full and half width, maximized, or hidden entirely. Hiding a panel also disables its corresponding background feature.
- Battery history: Framework Control now records up to seven days of battery percentage and charge/discharge power history. Available views range from the last hour through seven days, including a “Since last charge” option.
- Per-fan calibration: Multi-fan systems now get an individual duty-to-RPM calibration for each fan. Calibration waits longer for fans to stabilize, and the curve editor uses the appropriate calibration for each fan’s live RPM overlay.
- Config synchronization: Changes are synchronized across open Framework Control tabs without polling.
- New themes and visual overhaul: The interface has been refreshed with a curated set of themes, improved panel layouts, cleaner graphs, and better colors.
- Responsive UI improvements: A lot of smaller layout and overflow problems have been fixed, particularly on narrower windows.
- Accessibility and modal fixes: Settings and Logs no longer hide behind the header, Escape closes dialogs, and keyboard navigation has been improved.
- Linux packaging fix: The AUR build dependencies have been corrected so the web interface and service can build properly.
Important: Existing fan calibrations will need to be recorded again because calibration is now stored separately for each fan.
Framework Control is available for Linux and Windows and supports Framework Laptop 12, 13, 16, and Framework Desktop where the underlying framework_tool functionality is available.
Download, installation instructions, and source:
https://github.com/ozturkkl/framework-control
Thank you to everyone who has tested the app, reported issues, and suggested improvements. A surprising number of features in this release came directly from community feedback.
A few people have also asked whether there is a way to support the project. I recently got laid off, so I’ve finally set one up. Framework Control will remain free and open source, but if it has been useful to you and you would like to support its continued development, you can do so here:
[GitHub Sponsors or support link]
Either way, using the app, sharing it, and reporting issues are all greatly appreciated. Let me know what you think of the update!


Reddit draft
Suggested title:
Framework Control update: Custom dashboards, battery history, per-fan calibration, new themes, and more [Linux/Windows]

Post body:
Hey everyone!
I’ve been working on a fairly large Framework Control update, and it’s almost ready. Here’s a quick demo:
[NATIVE VIDEO]
The biggest additions are:
- Drag, resize, maximize, reorder, or hide dashboard panels
- Up to seven days of battery percentage and charge/discharge history
- A “Since last charge” battery-history view
- More accurate per-fan calibration on multi-fan systems
- Live config synchronization between open tabs
- A redesigned interface with new Light, Dark, Extra Dark, Retro, Synthwave, Coffee, Terminal, and System themes
- A large number of responsive-layout, graph, modal, and accessibility fixes
One important upgrade note: existing fan calibrations will need to be recorded again because calibration is now stored separately for each fan.
Framework Control is a free and open-source local app for controlling and monitoring supported Framework devices on Linux and Windows. It includes fan curves, sensors, power controls, battery controls, telemetry history, and more.
GitHub and installation instructions:
https://github.com/ozturkkl/framework-control
Full community discussion:
https://community.frame.work/t/framework-control-linux-windows-a-tool-for-fan-control-curve-sensors-power-control-battery-information/74533
A few users have asked me to add a way to support the project. I was recently laid off, so I’ve finally set one up. This is completely optional and the app will remain free and open source:
[support link]
Feedback and bug reports are very welcome!