// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{Application, InstallDir, Trampoline};
use std::{
    io::Error as IOError,
    ops::Deref,
    sync::atomic::{AtomicBool, Ordering},
};

static IS_BUNDLED: AtomicBool = AtomicBool::new(false);

pub struct Retained<T>(T);
impl<T> Retained<T> {
    pub fn new(inner: T) -> Self {
        Retained(inner)
    }
}
impl<T> Deref for Retained<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

pub struct NSApplication;
#[allow(non_snake_case)]
impl NSApplication {
    pub fn sharedApplication(mtm: MainThreadMarker) -> Retained<Self> {
        let _ = mtm;
        Retained(Self)
    }
}

pub struct NSBundle;
#[allow(non_snake_case)]
impl NSBundle {
    pub fn mainBundle() -> Retained<Self> {
        Retained(Self)
    }
}

pub fn has_bundle_identifier(_bundle: &NSBundle) -> bool {
    IS_BUNDLED.load(Ordering::Relaxed)
}

#[derive(Copy, Clone)]
pub struct MainThreadMarker;
impl MainThreadMarker {
    pub fn new() -> Option<Self> {
        Some(Self)
    }
}

pub fn bundle(trampoline: &Trampoline, _location: InstallDir) -> Result<Application, IOError> {
    IS_BUNDLED.store(true, Ordering::Relaxed);
    Ok(Application::new(
        trampoline.name.clone(),
        trampoline.ident.clone(),
        NSBundle::mainBundle(),
    ))
}

// End of File
