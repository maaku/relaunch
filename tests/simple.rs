// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use relaunch;

use std::io::Error as IOError;

fn main() -> Result<(), IOError> {
    // Bundle and relaunch the application in a temporary directory.
    let _app =
        relaunch::Trampoline::new("re-Test-Simple", "com.github.maaku.relauncher.tests.Simple")
            .bundle(relaunch::InstallDir::Temp)?;
    // Check that we are bundled.
    assert!(relaunch::Trampoline::is_bundled());
    Ok(())
}

// End of File
