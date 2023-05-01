/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::io::Error;

fn main() -> Result<(), Error> {
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-arg=-lbz2");
        println!("cargo:rustc-link-arg=-lpng16");
        println!("cargo:rustc-link-arg=-lbrotlidec");
        println!("cargo:rustc-link-arg=-lEGL");
    }
    Ok(())
}
