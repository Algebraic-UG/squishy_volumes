// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use plotters::prelude::*;
use squishy_volumes_core::kernels::{kernel_cubic, kernel_linear, kernel_quadratic};
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
                .map(|x| (x, kernel_linear(x.into()) as f32)),
            &RED,
        ))?
        .label("linear")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
    chart
        .draw_series(LineSeries::new(
            (-250..=250)
                .map(|x| x as f32 / 50.0)
                .map(|x| (x, kernel_quadratic(x.into()) as f32)),
            &GREEN,
        ))?
        .label("quadratic")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));
    chart
        .draw_series(LineSeries::new(
            (-250..=250)
                .map(|x| x as f32 / 50.0)
                .map(|x| (x, kernel_cubic(x.into()) as f32)),
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
