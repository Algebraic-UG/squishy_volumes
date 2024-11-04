// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

fn main() {
    // Calling `build_info_build::build_script` collects all data and makes it available to `build_info::build_info!`
    // and `build_info::format!` in the main program.
    build_info_build::build_script();
}
