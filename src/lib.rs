// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Relaunch is a library for bundling and relaunching a macOS application, in
//! order to access OS features that are only available to app bundles and not
//! command-line applications.

use std::io::Error as IOError;
use std::io::Write;
use std::path::{Path, PathBuf};

use objc2::rc::Retained;
use objc2_app_kit::NSApplication;
use objc2_foundation::{MainThreadMarker, NSBundle};

// The relaunch crate is only needed on the macOS platform, but gating
// dependencies by build configuration is not something that comes naturally
// to Cargo.  So we want to allow the crate to be built on other platforms,
// but only link to the necessary Cocoa frameworks on macOS.
//
// The caller is required to gate the use of the relaunch crate by platform,
// otherwise runtime errors will be encountered.
#[cfg(target_os = "macos")]
#[link(name = "Foundation", kind = "framework")] // For NSBundle
#[link(name = "AppKit", kind = "framework")] // For NSApplication
extern "C" {}

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
            bundle.bundleIdentifier().and_then(|_| Some(bundle))
        }
    }
    /// Checks if the running process is an applicaiton bundle.
    pub fn is_bundled() -> bool {
        Self::get_bundle().is_some()
    }

    pub fn bundle(&self, location: InstallDir) -> Result<Application, IOError> {
        if let Some(bundle) = Self::get_bundle() {
            return Ok(Application::new(
                self.name.clone(),
                self.ident.clone(),
                bundle,
            ));
        }

        let install_path = match location {
            InstallDir::Temp => std::env::temp_dir(),
            InstallDir::SystemApplications => PathBuf::from("/Applications"),
            InstallDir::UserApplications => dirs::home_dir().unwrap().join("Applications"),
            InstallDir::Custom(path) => std::fs::canonicalize(path)?,
        };
        let bundle_path = install_path.join(&format!("{}.app", self.name));
        let contents_path = Path::new(&bundle_path).join("Contents");
        let macos_path = contents_path.clone().join("MacOS");
        let resources_path = contents_path.clone().join("Resources");
        let plist = contents_path.clone().join("Info.plist");

        let src_exe = std::env::current_exe()?;
        let exe_name = src_exe
            .file_name()
            .expect("Could not determine executable name for current process.")
            .to_str()
            .expect("Could not convert executable name to string.");
        let dst_exe = macos_path.clone().join(&exe_name);

        // Remove the app bundle if it already exists (e.g. from a previous run).
        if bundle_path.try_exists()? {
            std::fs::remove_dir_all(&bundle_path)?;
        }
        // Create the bundle directory structure.
        std::fs::create_dir_all(&macos_path)?;
        std::fs::create_dir_all(&resources_path)?;
        // Copy the executable to the MacOS directory.
        std::fs::copy(&src_exe, &dst_exe)?;

        // Write Info.plist
        let mut f = std::fs::File::create(&plist)?;
        write!(&mut f, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
        write!(&mut f, "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n")?;
        write!(&mut f, "<plist version=\"1.0\">\n")?;
        write!(&mut f, "<dict>\n")?;
        write!(&mut f, "\t<key>CFBundleName</key>\n")?;
        write!(&mut f, "\t<string>{}</string>\n", self.name)?;
        write!(&mut f, "\t<key>CFBundleDisplayName</key>\n")?;
        write!(&mut f, "\t<string>{}</string>\n", self.name)?;
        write!(&mut f, "\t<key>CFBundleIdentifier</key>\n")?;
        write!(&mut f, "\t<string>{}</string>\n", self.ident)?;
        write!(&mut f, "\t<key>CFBundleExecutable</key>\n")?;
        write!(&mut f, "\t<string>{}</string>\n", exe_name)?;
        write!(&mut f, "\t<key>CFBundleShortVersionString</key>\n")?;
        write!(&mut f, "\t<string>{}</string>\n", self.version)?;
        write!(&mut f, "\t<key>CFBundleSupportedPlatforms</key>\n")?;
        write!(&mut f, "\t<array>\n")?;
        write!(&mut f, "\t\t<string>MacOSX</string>\n")?;
        write!(&mut f, "\t</array>\n")?;
        write!(&mut f, "\t<key>CFBundleVersion</key>\n")?;
        write!(&mut f, "\t<string>{}</string>\n", self.version)?;
        write!(&mut f, "\t<key>NSPrincipalClass</key>\n")?;
        write!(&mut f, "\t<string>NSApplication</string>\n")?;
        write!(&mut f, "\t<key>NSHighResolutionCapable</key>\n")?;
        write!(&mut f, "\t<true/>\n")?;
        write!(&mut f, "\t<key>CFBundleInfoDictionaryVersion</key>\n")?;
        write!(&mut f, "\t<string>6.0</string>\n")?;
        write!(&mut f, "\t<key>CFBundlePackageType</key>\n")?;
        write!(&mut f, "\t<string>APPL</string>\n")?;
        write!(&mut f, "\t<key>CFBundleSignature</key>\n")?;
        write!(&mut f, "\t<string>????</string>\n")?;
        write!(&mut f, "\t<key>LSMinimumSystemVersion</key>\n")?;
        write!(&mut f, "\t<string>10.10.0</string>\n")?;
        write!(&mut f, "</dict>\n")?;
        write!(&mut f, "</plist>\n")?;

        // Launch newly created bundle
        let status = std::process::Command::new(dst_exe).spawn()?.wait()?;
        match status.code() {
            // If the app exited with exit code, return that code.
            Some(code) => std::process::exit(code),
            // Otherwise the app was terminated by a signal.  We should find
            // some way to propagate that signal, but for now we just exit
            // with code 125 (the highest user-defined POSIX exit code) to
            // indicate an error.
            None => std::process::exit(125),
        }
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
