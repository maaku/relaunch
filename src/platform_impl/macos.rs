// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{Application, InstallDir, Trampoline};
use std::{
    io::{Error as IOError, Write},
    path::{Path, PathBuf},
};

pub use objc2::rc::Retained;
pub use objc2_app_kit::NSApplication;
pub use objc2_foundation::{MainThreadMarker, NSBundle};

// The relaunch crate is only needed on the macOS platform, but gating
// dependencies by build configuration is not something that comes naturally
// to Cargo.  So we want to allow the crate to be built on other platforms,
// but only link to the necessary Cocoa frameworks on macOS.
//
// The caller is required to gate the use of the relaunch crate by platform,
// otherwise runtime errors will be encountered.
#[link(name = "AppKit", kind = "framework")] // For NSApplication
extern "C" {}
#[link(name = "Foundation", kind = "framework")] // For NSBundle
extern "C" {}

pub fn bundle(trampoline: &Trampoline, location: InstallDir) -> Result<Application, IOError> {
    if let Some(bundle) = Trampoline::get_bundle() {
        return Ok(Application::new(
            trampoline.name.clone(),
            trampoline.ident.clone(),
            bundle,
        ));
    }

    let install_path = match location {
        InstallDir::Temp => std::env::temp_dir(),
        InstallDir::SystemApplications => PathBuf::from("/Applications"),
        InstallDir::UserApplications => dirs::home_dir().unwrap().join("Applications"),
        InstallDir::Custom(path) => std::fs::canonicalize(path)?,
    };
    let bundle_path = install_path.join(format!("{}.app", trampoline.name));
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
    let dst_exe = macos_path.clone().join(exe_name);

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
    writeln!(&mut f, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(&mut f, "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">")?;
    writeln!(&mut f, "<plist version=\"1.0\">")?;
    writeln!(&mut f, "<dict>")?;
    writeln!(&mut f, "\t<key>CFBundleName</key>")?;
    writeln!(&mut f, "\t<string>{}</string>", trampoline.name)?;
    writeln!(&mut f, "\t<key>CFBundleDisplayName</key>")?;
    writeln!(&mut f, "\t<string>{}</string>", trampoline.name)?;
    writeln!(&mut f, "\t<key>CFBundleIdentifier</key>")?;
    writeln!(&mut f, "\t<string>{}</string>", trampoline.ident)?;
    writeln!(&mut f, "\t<key>CFBundleExecutable</key>")?;
    writeln!(&mut f, "\t<string>{}</string>", exe_name)?;
    writeln!(&mut f, "\t<key>CFBundleShortVersionString</key>")?;
    writeln!(&mut f, "\t<string>{}</string>", trampoline.version)?;
    writeln!(&mut f, "\t<key>CFBundleSupportedPlatforms</key>")?;
    writeln!(&mut f, "\t<array>")?;
    writeln!(&mut f, "\t\t<string>MacOSX</string>")?;
    writeln!(&mut f, "\t</array>")?;
    writeln!(&mut f, "\t<key>CFBundleVersion</key>")?;
    writeln!(&mut f, "\t<string>{}</string>", trampoline.version)?;
    writeln!(&mut f, "\t<key>NSPrincipalClass</key>")?;
    writeln!(&mut f, "\t<string>NSApplication</string>")?;
    writeln!(&mut f, "\t<key>NSHighResolutionCapable</key>")?;
    writeln!(&mut f, "\t<true/>")?;
    writeln!(&mut f, "\t<key>CFBundleInfoDictionaryVersion</key>")?;
    writeln!(&mut f, "\t<string>6.0</string>")?;
    writeln!(&mut f, "\t<key>CFBundlePackageType</key>")?;
    writeln!(&mut f, "\t<string>APPL</string>")?;
    writeln!(&mut f, "\t<key>CFBundleSignature</key>")?;
    writeln!(&mut f, "\t<string>????</string>")?;
    writeln!(&mut f, "\t<key>LSMinimumSystemVersion</key>")?;
    writeln!(&mut f, "\t<string>10.10.0</string>")?;
    writeln!(&mut f, "</dict>")?;
    writeln!(&mut f, "</plist>")?;

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

// End of File
