// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Relaunch is a library for bundling and relaunching a macOS application, in
//! order to access OS features that are only available to app bundles and not
//! command-line applications.

use std::{io::Error as IOError, path::PathBuf, process::ExitCode};

mod platform_impl;
use platform_impl::{MainThreadMarker, NSApplication, NSBundle, Retained};

extern crate dirs;

/// Where to save the generated app bundle.
pub enum InstallDir {
    /// Save the app bundle in a system-defined temporary directory.
    Temp,
    /// Save the app bundle in the system-wide `Applications` directory.
    SystemApplications,
    /// Save the app bundle in the user-specific `Applications` directory.
    UserApplications,
    /// Save the app bundle custom directory specified by the caller.
    Custom(PathBuf),
}

/// The applicaiton relauncher, which is used to build the app bundle, launch
/// it as a subprocess, and then wait for it to exit.  Or if we are already
/// running from within an app bundle, do nothing.
pub struct Trampoline {
    /// The name of the application as shown to the user, and also the name of
    /// the app bundle in the filesystem.
    name: String,
    /// The unique identifier for the application, which should be in reverse
    /// DNS format, e.g. "org.example.MyApp", and must contain only
    /// alpha-numeric characters, '-', and '.'.
    ident: String,
    /// The version number of the application, which should be in the format
    /// "major.minor.patch", e.g. "1.0.0".
    version: String,
}

impl Trampoline {
    pub fn new(name: &str, ident: &str) -> Self {
        Trampoline {
            name: name.to_string(),
            ident: ident.to_string(),
            // FIXME: This defaults to the relaunch crate version, not the
            //        version of the binary being built.  This is almost
            //        certainly not what the user wants.
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Set the name of the app bundle.  Overrides value provided to `new()`.
    pub fn name(&mut self, name: &str) -> &mut Self {
        self.name = name.to_string();
        self
    }
    /// Set the app bundle ID.  Overrides value provided to `new()`.
    pub fn ident(&mut self, ident: &str) -> &mut Self {
        self.ident = ident.to_string();
        self
    }
    /// Set the app bundle version.  Overrides the default value pulled from
    /// `CARGO_PKG_VERSION`.
    pub fn version(&mut self, version: &str) -> &mut Self {
        self.version = version.to_string();
        self
    }

    /// Get a reference to the NSBundle class, which we will use to query if
    /// our process is running as an app bundle.
    fn get_bundle() -> Option<Retained<NSBundle>> {
        // Get a reference to the main bundle.  This can fail (return nil)
        // if we are not running as an app bundle, but it is not
        // guaranteed.
        let bundle = NSBundle::mainBundle();
        unsafe {
            // Get a NSString copy of the CFBundleIdentifier key from the
            // bundle's Info.plist.  This for sure will only work if we are
            // running from within a properly configured application bundle.
            // Otherwise, return None.
            bundle.bundleIdentifier().map(|_| bundle)
        }
    }
    /// Checks if the running process is an applicaiton bundle.
    pub fn is_bundled() -> bool {
        Self::get_bundle().is_some()
    }

    pub fn bundle(&self, location: InstallDir) -> Result<Application, IOError> {
        platform_impl::bundle(self, location)
    }

    #[cfg(feature = "winit")]
    pub fn run_once<T>(&self, location: InstallDir, cb: T)
    where
        T: FnOnce(&Application) -> ExitCode + 'static,
    {
        use winit::{
            application::ApplicationHandler,
            event::WindowEvent,
            event_loop::{ActiveEventLoop, EventLoop},
            window::WindowId,
        };

        #[allow(clippy::type_complexity)]
        struct WinitApp {
            relaunch_app: Application,
            cb: Option<Box<dyn FnOnce(&Application) -> ExitCode + 'static>>,
        }
        impl ApplicationHandler for WinitApp {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                // Required to be implemented, but we don't need to do anything.
                let _ = event_loop;
            }

            fn window_event(
                &mut self,
                event_loop: &ActiveEventLoop,
                window_id: WindowId,
                event: WindowEvent,
            ) {
                // Required to be implemented, but we don't need to do anything.
                let _ = (event_loop, window_id, event);
            }

            // We will run the user callback once all OS events have been processed, in the
            // about_to_wait event handler.
            fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
                if let Some(cb) = self.cb.take() {
                    // Run the user's callback.
                    let _ = cb(&self.relaunch_app);
                    // Terminate the application.
                    event_loop.exit();
                }
            }
        }

        // We don't launch any windows, so we aren't a graphical application.
        // There should be an item in the dock, but no windows or menubar.
        let event_loop = EventLoop::new().expect("Failed to create event loop");

        // Relaunch the application as a bundled application.
        let relaunch_app = self.bundle(location).unwrap_or_else(|error| {
            eprintln!("Application relaunch failed: {}", error);
            // Something seriously wrong happened.  Bail out.
            std::process::exit(1);
        });

        let mut winit_app = WinitApp {
            relaunch_app,
            cb: Some(Box::new(cb)),
        };

        if let Err(err) = event_loop.run_app(&mut winit_app) {
            eprintln!("Event loop terminated with error: {}", err);
        };
    }
}

/// The application, including pointers to the `[NSBundle mainBundle]` and
/// `[NSApplication sharedApplication]` instances for the relaunched app
/// bundle.
pub struct Application {
    /// The name of the application, as shown in the Dock and menubar.
    pub name: String,
    /// The bundle identifier of the application, which should be a
    /// reverse-DNS style unique string identifier using only alpha-numeric
    /// characters, the '.' dot, and '-' hyphen.
    pub ident: String,
    /// The path to the app bundle on the filesytem from which this process is
    /// running.  Note that if the app was already running from an application
    /// bundle, this might not be the same directory in which the app bundle
    /// would have been generated.
    pub bundle_path: PathBuf,
    /// A reference to the `[NSBundle mainBundle]` instance for the app
    /// bundle.
    pub bundle: Retained<NSBundle>,
    /// A reference to the `[NSApplication sharedApplication]` instance for
    /// the application.
    pub app: Retained<NSApplication>,
}

impl Application {
    fn new(name: String, ident: String, bundle: Retained<NSBundle>) -> Self {
        // Get the path to app bundle from which we are running.
        let mut bundle_path =
            std::env::current_exe().expect("Could not determine path to current executable.");
        bundle_path.pop(); // [exe]
        bundle_path.pop(); // MacOS
        bundle_path.pop(); // Contents

        // Establish that we are running on the main thread.
        let mtm =
            MainThreadMarker::new().expect("Must call Application::new() from the main thread.");

        // Get a reference to the shared application instance.
        let app = NSApplication::sharedApplication(mtm);

        // Return the new Application instance.
        Self {
            name,
            ident,
            bundle_path,
            bundle,
            app,
        }
    }
}

// End of File
