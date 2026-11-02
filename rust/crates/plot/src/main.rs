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

use plotters::prelude::*;
use squishy_volumes_cpu::{kernel_cubic, kernel_linear, kernel_quadratic};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new("out.png", (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption("kernels", ("sans-serif", 50).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(-2.5f32..2.5f32, -0.1f32..1f32)?;

    chart.configure_mesh().draw()?;

    chart
        .draw_series(LineSeries::new(
            (-250..=250)
                .map(|x| x as f32 / 50.0)
                .map(|x| (x, kernel_linear(x))),
            &RED,
        ))?
        .label("linear")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    chart
        .draw_series(LineSeries::new(
            (-250..=250)
                .map(|x| x as f32 / 50.0)
                .map(|x| (x, kernel_quadratic(x))),
            &GREEN,
        ))?
        .label("quadratic")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));
    chart
        .draw_series(LineSeries::new(
            (-250..=250)
                .map(|x| x as f32 / 50.0)
                .map(|x| (x, kernel_cubic(x))),
            &BLUE,
        ))?
        .label("cubic")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}
