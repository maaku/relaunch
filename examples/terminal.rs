// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

extern crate relaunch;

use std::{io::Write, process::ExitCode};

fn main() {
    let app = match relaunch::Trampoline::new("re-Terminal", "com.github.maaku.relauncher.Terminal")
        .version("1.0.0")
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

    app.run_once(|| {
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
        ExitCode::SUCCESS
    })
}

// End of File
