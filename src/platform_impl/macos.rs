// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{Application, InstallDir, Trampoline};
use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};
use signal_hook::{
    consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};
use std::{
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hasher},
    io::{Error as IOError, ErrorKind, Write},
    path::{Path, PathBuf},
    process::ExitStatus,
};

pub use objc2::rc::Retained;
pub use objc2_app_kit::NSApplication;
pub use objc2_foundation::{MainThreadMarker, NSBundle};

use objc2_foundation::ns_string;

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

/// Checks whether the bundle's Info.plist provides a bundle identifier.
pub fn has_bundle_identifier(bundle: &NSBundle) -> bool {
    bundle
        .infoDictionary()
        .is_some_and(|info| info.get(ns_string!("CFBundleIdentifier")).is_some())
}

pub fn bundle(trampoline: &Trampoline, location: InstallDir) -> Result<Application, IOError> {
    if let Some(bundle) = Trampoline::get_bundle() {
        return Ok(Application::new(
            trampoline.name.clone(),
            trampoline.ident.clone(),
            bundle,
        ));
    }

    // Disposable bundles each get a directory of their own, so that multiple
    // instances do not interfere with one another, which is removed once the
    // application exits.
    let (install_path, disposable) = match location {
        InstallDir::Temp => {
            let parent = dirs::cache_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join(&trampoline.ident);
            (create_unique_dir(&parent)?, Some(parent))
        }
        InstallDir::SystemApplications => (PathBuf::from("/Applications"), None),
        InstallDir::UserApplications => (dirs::home_dir().unwrap().join("Applications"), None),
        InstallDir::Custom(path) => (std::fs::canonicalize(path)?, None),
    };

    let status = launch(trampoline, &install_path);
    if let Some(parent) = disposable {
        let _ = std::fs::remove_dir_all(&install_path);
        // Only succeeds if no other instance is still using it.
        let _ = std::fs::remove_dir(&parent);
    }

    match status?.code() {
        // If the app exited with exit code, return that code.
        Some(code) => std::process::exit(code),
        // Otherwise the app was terminated by a signal.  We should find
        // some way to propagate that signal, but for now we just exit
        // with code 125 (the highest user-defined POSIX exit code) to
        // indicate an error.
        None => std::process::exit(125),
    }
}

/// Creates a new, empty directory within `parent`, named after this process
/// and a random nonce.
fn create_unique_dir(parent: &Path) -> Result<PathBuf, IOError> {
    let pid = std::process::id();
    loop {
        std::fs::create_dir_all(parent)?;
        let nonce = RandomState::new().build_hasher().finish();
        let dir = parent.join(format!("{pid}-{nonce:016x}"));
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            // The name is already taken, or the parent directory was removed
            // by an exiting instance in the meantime.
            Err(error)
                if matches!(error.kind(), ErrorKind::AlreadyExists | ErrorKind::NotFound) => {}
            Err(error) => return Err(error),
        }
    }
}

/// Builds the app bundle within `install_path`, then runs it and waits for it
/// to exit.
fn launch(trampoline: &Trampoline, install_path: &Path) -> Result<ExitStatus, IOError> {
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
    let signals = Signals::new([SIGHUP, SIGINT, SIGQUIT, SIGTERM])?;
    let mut child = std::process::Command::new(dst_exe).spawn()?;
    relay_signals(signals, Pid::from_raw(child.id() as i32));
    child.wait()
}

/// Handles the signals caught by `signals` on behalf of the relaunched
/// application at `pid`, including any caught before it was started.
///
/// Catching these signals keeps the trampoline alive until the application
/// exits, so that it can clean up after it.  Signals generated by the terminal
/// are delivered to the application directly, as it is in the same process
/// group, so only termination requests need to be relayed.  Unlike ignored
/// signals, caught signals revert to their default action in the application.
fn relay_signals(mut signals: Signals, pid: Pid) {
    std::thread::spawn(move || {
        for signal in signals.forever() {
            if signal == SIGTERM {
                let _ = kill(pid, Signal::SIGTERM);
            }
        }
    });
}

// End of File
