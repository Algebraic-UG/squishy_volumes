// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

fn main() {
    println!("cargo::rustc-cfg=use_f64");
    println!("cargo::rustc-check-cfg=cfg(use_f64)");
}
