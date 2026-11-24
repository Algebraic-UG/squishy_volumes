// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#[test]
fn check_version() {
    crate::initialize();
    assert_eq!(
        crate::CombinedBuildInfo::new()
            .unwrap()
            .wrapper
            .crate_info
            .version
            .to_string(),
        squishy_volumes_file_util::version_string(),
    );
}

#[cfg(feature = "hot_reload")]
#[test]
fn unavailable_state_reports_reason() {
    let error = crate::hot_reload::HotState::Unavailable("rebuild the extension".to_string())
        .context()
        .err()
        .unwrap();
    assert_eq!(error.to_string(), "rebuild the extension");
}

#[cfg(feature = "hot_reload")]
#[test]
fn mismatched_api_build_stamp_asks_for_rebuild() {
    use crate::hot_reload::check_api_build_stamp;
    assert!(check_api_build_stamp(1, Some(1)).is_ok());
    for found in [Some(2), None] {
        let error = check_api_build_stamp(1, found).unwrap_err();
        assert!(error.contains("Rebuild the extension"), "{error}");
    }
}
