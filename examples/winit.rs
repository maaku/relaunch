// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

extern crate relaunch;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

fn main() {
    // First create the winit event loop.  This *must* come before calling out to relaunch.
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    // Re-launch the application as a bundled application.
    match relaunch::Trampoline::new("re-winit", "com.github.maaku.relauncher.winit")
        .bundle(relaunch::InstallDir::Temp)
    {
        Err(error) => {
            println!("Application relaunch failed: {}", error);
            // Something seriously wrong happened.  Bail out.
            std::process::exit(1);
        }
        Ok(app) => {
            println!(
                "Application relaunched successfully from {}",
                app.bundle_path.to_str().unwrap()
            );
        }
    };

    // We are now running as a bundled application.
    assert!(relaunch::Trampoline::is_bundled());

    // Create the window.
    let window = winit::window::WindowBuilder::new()
        .with_title("re-winit")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .expect("Failed to create window");

    // Run the event loop.
    if let Err(err) = event_loop.run(move |event, window_target| {
        // ControlFlow::Wait pauses the event loop if no events are available
        // to process.  So long as the contents of the window are not changing,
        // we only need to redraw after processing all events.
        window_target.set_control_flow(ControlFlow::Wait);

        #[allow(clippy::collapsible_match, clippy::single_match)]
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::RedrawRequested => {
                    // Redraw the application.
                    //...
                }

                WindowEvent::CloseRequested => {
                    // Terminate the application.
                    window_target.exit();
                }

                // Ignore all other `WindowEvent`'s.
                _ => (),
            },
            Event::AboutToWait => {
                // Application update code
                let redraw_required = false;
                //...

                // Queue a RedrawRequested event if the window contents have
                // changed.
                if redraw_required {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }) {
        eprintln!("Event loop terminated with error: {}", err);
    };
}

// End of File
