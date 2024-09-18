// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A very, very simple test to make sure that `Trampoline::is_bundled()`
//! returns `false` when run from the command line (e.g. via `cargo test`). We
//! cannot make this check as part of any other integration test, as the test
//! would fail when the process is relaunched as an app bundle. Therefore this
//! simple test only checks the state prior to bundling, and does not perform
//! the relaunch step.

#[test]
fn null() {
    // We are assuming that this process is being run from the command line,
    // e.g. by `cargo test`.  Therefore, we should not be bundled.
    assert!(!relaunch::Trampoline::is_bundled());
}

// End of File
