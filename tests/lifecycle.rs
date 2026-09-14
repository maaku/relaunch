// Copyright (c) 2023-2024 by Mark Friedenbach <mark@friedenbach.org>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Checks the lifecycle of app bundles relaunched from `InstallDir::Temp`:
//! each instance runs from a directory of its own, which is removed once the
//! application exits, however it exits.
//!
//! This test binary is both the test driver and the application that gets
//! relaunched, depending on whether `HELPER_IDENT` is set in its environment.
//! The relaunched application reports that it is running by writing its
//! process ID and bundle path to stdout, then blocks until the driver writes
//! the exit code it should use to stdin (or sends it a signal).

#[cfg(target_os = "macos")]
fn main() {
    macos::main();
}

// Relaunching does nothing on other platforms, so there is nothing to test.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod macos {
    use nix::{
        sys::signal::{kill, killpg, Signal},
        unistd::Pid,
    };
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::process::CommandExt,
        path::{Path, PathBuf},
        process::{Child, ChildStdin, Command, ExitStatus, Stdio},
    };

    /// Environment variable carrying the bundle identifier to the application.
    const HELPER_IDENT: &str = "RELAUNCH_TEST_LIFECYCLE_IDENT";

    const NAME: &str = "re-Test-Lifecycle";

    pub fn main() {
        if let Ok(ident) = std::env::var(HELPER_IDENT) {
            return application(&ident);
        }

        exit_code_is_relayed();
        concurrent_instances();
        interrupt_process_group();
        terminate_trampoline();
    }

    fn application(ident: &str) {
        let app = relaunch::Trampoline::new(NAME, ident)
            .bundle(relaunch::InstallDir::Temp)
            .expect("Failed to relaunch as an app bundle");
        // Only the relaunched application gets this far.
        assert!(relaunch::Trampoline::is_bundled());

        println!("{} {}", std::process::id(), app.bundle_path.display());
        std::io::stdout().flush().unwrap();

        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        let code = line.trim().parse().expect("Expected an exit code");
        std::process::exit(code);
    }

    fn exit_code_is_relayed() {
        let ident = ident("exit-code");
        let mut instance = Instance::launch(&ident);
        instance.assert_running(&ident);
        instance.tell_exit(7);
        let (status, instance) = instance.wait();
        assert_eq!(status.code(), Some(7));
        instance.assert_cleaned_up(&ident);
        println!("test exit_code_is_relayed ... ok");
    }

    fn concurrent_instances() {
        let ident = ident("concurrent");
        let mut first = Instance::launch(&ident);
        let mut second = Instance::launch(&ident);
        first.assert_running(&ident);
        second.assert_running(&ident);
        assert_ne!(first.bundle_path.parent(), second.bundle_path.parent());

        first.tell_exit(0);
        let (status, first) = first.wait();
        assert_eq!(status.code(), Some(0));
        first.assert_cleaned_up_instance();
        // The other instance, and so the shared parent directory, is unaffected.
        second.assert_running(&ident);

        second.tell_exit(0);
        let (status, second) = second.wait();
        assert_eq!(status.code(), Some(0));
        second.assert_cleaned_up(&ident);
        println!("test concurrent_instances ... ok");
    }

    fn interrupt_process_group() {
        let ident = ident("interrupt");
        let instance = Instance::launch(&ident);
        instance.assert_running(&ident);
        // As sent by the terminal on ^C, to both trampoline and application.
        killpg(instance.trampoline_pid(), Signal::SIGINT).unwrap();
        let (status, instance) = instance.wait();
        assert_eq!(status.code(), Some(125));
        instance.assert_cleaned_up(&ident);
        println!("test interrupt_process_group ... ok");
    }

    fn terminate_trampoline() {
        let ident = ident("terminate");
        let instance = Instance::launch(&ident);
        instance.assert_running(&ident);
        kill(instance.trampoline_pid(), Signal::SIGTERM).unwrap();
        let (status, instance) = instance.wait();
        assert_eq!(status.code(), Some(125));
        instance.assert_cleaned_up(&ident);
        println!("test terminate_trampoline ... ok");
    }

    /// A bundle identifier unique to this test run and scenario, so that
    /// leftovers from elsewhere cannot affect the outcome.
    fn ident(scenario: &str) -> String {
        format!(
            "com.github.maaku.relauncher.tests.Lifecycle.{}.{scenario}",
            std::process::id()
        )
    }

    /// The directory `InstallDir::Temp` bundles with this identifier go in.
    fn bundles_dir(ident: &str) -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(ident)
    }

    fn is_alive(pid: Pid) -> bool {
        kill(pid, None).is_ok()
    }

    /// A relaunched application which has reported that it is running.
    struct Instance {
        trampoline: Child,
        stdin: Option<ChildStdin>,
        /// Process ID of the relaunched application.
        pid: Pid,
        bundle_path: PathBuf,
    }

    impl Instance {
        /// Starts this binary as an application to be relaunched, and blocks
        /// until the relaunched application reports that it is running.
        fn launch(ident: &str) -> Self {
            let mut trampoline = Command::new(std::env::current_exe().unwrap())
                .env(HELPER_IDENT, ident)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                // Keeps signals sent to the process group away from the driver.
                .process_group(0)
                .spawn()
                .expect("Failed to start the trampoline");

            // Reaches end of file instead if the application never starts.
            let mut report = String::new();
            BufReader::new(trampoline.stdout.take().unwrap())
                .read_line(&mut report)
                .unwrap();
            let (pid, bundle_path) = report
                .trim_end()
                .split_once(' ')
                .unwrap_or_else(|| panic!("Application failed to start: {report:?}"));

            Self {
                stdin: trampoline.stdin.take(),
                trampoline,
                pid: Pid::from_raw(pid.parse().unwrap()),
                bundle_path: PathBuf::from(bundle_path),
            }
        }

        fn trampoline_pid(&self) -> Pid {
            Pid::from_raw(self.trampoline.id() as i32)
        }

        fn tell_exit(&mut self, code: i32) {
            writeln!(self.stdin.as_mut().unwrap(), "{code}").unwrap();
        }

        /// Blocks until the trampoline exits.
        fn wait(mut self) -> (ExitStatus, Self) {
            let status = self.trampoline.wait().unwrap();
            (status, self)
        }

        fn assert_running(&self, ident: &str) {
            assert!(is_alive(self.pid));
            assert!(self.bundle_path.is_dir());
            let dir = self.bundle_path.parent().unwrap();
            assert_eq!(dir.parent(), Some(bundles_dir(ident).as_path()));
        }

        fn assert_cleaned_up_instance(&self) {
            assert!(!is_alive(self.pid), "Application is still running");
            let dir = self.bundle_path.parent().unwrap();
            assert!(!dir.exists(), "{} was not removed", dir.display());
        }

        fn assert_cleaned_up(&self, ident: &str) {
            self.assert_cleaned_up_instance();
            let dir: &Path = &bundles_dir(ident);
            assert!(!dir.exists(), "{} was not removed", dir.display());
        }
    }
}

// End of File
