// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

extern crate relaunch;

fn main() {
    let _ = match relaunch::Trampoline::new("re-winit", "com.github.maaku.relauncher.winit")
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
            app
        }
    };

    // We are now running as a bundled application.
    assert!(relaunch::Trampoline::is_bundled());

    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("re-winit")
        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .expect("Failed to create window");
    event_loop.run(move |event, _, control_flow| {
        // ControlFlow::Wait pauses the event loop if no events are available
        // to process.  So long as the contents of the window are not changing,
        // we only need to redraw after processing all events.
        *control_flow = winit::event_loop::ControlFlow::Wait;
        match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    // Terminate the application.
                    *control_flow = winit::event_loop::ControlFlow::ExitWithCode(0);
                }

                // Ignore all other `WindowEvent`'s.
                _ => (),
            },
            winit::event::Event::MainEventsCleared => {
                // Application update code
                let redraw_required = false;
                //...

                // Queue a RedrawRequested event if the window contents have
                // changed.
                if redraw_required {
                    window.request_redraw();
                }
            }
            winit::event::Event::RedrawRequested(_) => {
                // Redraw the application.
                //...
            }
            _ => (),
        }
    });
}

// End of File
