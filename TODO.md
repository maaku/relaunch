# Known Bugs and Possible Future Features

## Bugs

* The version field defaults to `CARGO_PKG_VERSION`, which is the version of
  the crate that contains the macro, which is `relaunch` itself.  What is
  desired is the version of the top-level crate which includes `relaunch` as a
  dependency.  From the cargo docs there doesn't appear to be any
  environmental variable which exports this information.  Perhaps a build
  script could be used to write the version to a file which is then read by
  the macro?  In any case, this ought to be investigated.

## Missing Features

* Right now a version string is taken directly from the user, and the common
  recommendation is to pull the version from the Cargo.toml file using
  `CARGO_PKG_VERSION`.  But actually there are some pretty strict requirements
  about how the version string can be structured, what minimum and maximum
  values each version component can take, as well as rules about alpha, beta,
  and release candidate suffixes.  The rules are all described in the Apple
  documentation for `NSApplication` and `NSBundle`.  Respecting these rules is
  required if you want your program to pass validation for the App Store.  It
  would be nice to automatically build a conformant version string from the
  crate version.
