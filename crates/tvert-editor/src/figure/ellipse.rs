use crate::{
    figure::Figure,
    viewport::{Viewport, translate_rectangle},
};
use iced::{
    Radians, Rectangle, Vector,
    widget::canvas::{Path, path::arc::Elliptical},
};
use std::{f32::consts::TAU, fmt::Debug};

/// A elliptical outline in captured-image pixel coordinates
#[derive(Debug)]
pub struct EllipseFigure {
    bounds: Rectangle,
}

impl EllipseFigure {
    pub const fn new(bounds: Rectangle) -> Self {
        Self { bounds }
    }
}

impl From<Rectangle> for EllipseFigure {
    fn from(bounds: Rectangle) -> Self {
        Self::new(bounds)
    }
}

impl Figure for EllipseFigure {
    fn path(&self, viewport: Viewport) -> Path {
        let bounds = viewport.canvas_rectangle(self.bounds);

        Path::new(|builder| {
            builder.ellipse(Elliptical {
                center: bounds.center(),
                radii: Vector::new(bounds.width / 2.0, bounds.height / 2.0),
                rotation: Radians(0.0),
                start_angle: Radians(0.0),
                end_angle: Radians(TAU),
            });
            builder.close();
        })
    }

    fn translated(&self, offset: Vector) -> Box<dyn Figure> {
        Box::new(Self::new(translate_rectangle(self.bounds, offset)))
    }
}
