// Copyright (c) 2023 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

extern crate relaunch;

use std::io::Write;

fn main() {
    let _ = match relaunch::Trampoline::new("re-Terminal", "com.github.maaku.relauncher.Terminal")
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

    // We don't launch any windows, so we aren't a graphical application.
    // There should be an item in the dock, but no windows or menubar.
    let event_loop = winit::event_loop::EventLoop::new();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Wait;
        match event {
            winit::event::Event::MainEventsCleared => {
                // Run the example code.
                run();
                // Terminate the application.
                *control_flow = winit::event_loop::ControlFlow::ExitWithCode(0);
            }
            _ => (),
        }
    });
}

fn run() {
    // Print a prompt and read a line of input:
    // > This should come before the prompt.
    // > Please enter your name: Harry Potter
    // > Hello, Harry Potter!
    print!("Please enter your name: ");
    eprintln!("This should come before the prompt.");
    std::io::stdout().flush().unwrap();
    let mut name = String::new();
    std::io::stdin()
        .read_line(&mut name)
        .expect("Failed to read line");
    if name.ends_with('\n') {
        name.pop();
    }
    println!("Hello, {}!", name);
}

// End of File
