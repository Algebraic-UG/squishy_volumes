// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[test]
fn check_version() {
    assert_eq!(
        crate::CombinedBuildInfo::new()
            .wrapper
            .crate_info
            .version
            .to_string(),
        squishy_volumes_file_util::version_string(),
    );
}
