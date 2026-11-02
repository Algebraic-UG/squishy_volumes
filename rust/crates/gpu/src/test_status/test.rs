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

use super::*;

#[test]
fn read_back_status() {
    let mut context = get_shared_context();
    let test_status = TestStatus::new(
        &mut context,
        Settings {
            workgroup_size: 64.try_into().unwrap(),
        },
    )
    .unwrap();
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = test_status
        .record(&mut context, &mut (&mut encoder).into(), Input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, context.status());
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let status: GpuStatus = download.to_vec().unwrap()[0];

    context.reset_status().unwrap();

    assert!(matches!(
        status.to_result(&context),
        Err(GpuError::Shader(GpuShaderError::TableTriesExceeded {
            reporting_shader: "test_status"
        })),
    ));
}
