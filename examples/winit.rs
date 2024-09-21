// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

extern crate relaunch;

use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, Size},
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

struct App {
    window: Option<Window>,
}

impl App {
    fn new() -> Self {
        App { window: None }
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if cause == StartCause::Init {
            // Application initialization code
            //...

            // Create the window.
            self.window = Some(
                event_loop
                    .create_window(
                        WindowAttributes::default()
                            .with_title("re-winit")
                            .with_inner_size(Size::Logical(LogicalSize::new(800.0, 600.0))),
                    )
                    .expect("Failed to create window"),
            );

            // ControlFlow::Wait pauses the event loop if no events are available
            // to process.  So long as the contents of the window are not changing,
            // we only need to redraw after processing all events.
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop; // unused

        // Application update code
        let redraw_required = false;
        //...

        // Queue a RedrawRequested event if the window contents have changed.
        if redraw_required {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let _ = window_id; // unused
        match event {
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //...
            }

            WindowEvent::CloseRequested => {
                // Terminate the application.
                event_loop.exit();
            }

            // Ignore all other `WindowEvent`'s.
            _ => (),
        }
    }
}

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

    // Run the event loop.
    let mut app = App::new();
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("Event loop terminated with error: {}", err);
    };
}

// End of File
