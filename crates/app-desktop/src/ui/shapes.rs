//! Material 3 Expressive's shapes, as M3E draws them: each a polygon of
//! [`POINTS`] points round its centre, one radius per angle, so that any
//! shape morphs into any other by easing its radii into the other's. They
//! are drawn as a fan of triangles from the centre (the cookies aren't
//! convex), filled with a colour or a cover.

use std::f32::consts::{FRAC_PI_2, TAU};

use egui::{Color32, Mesh, Pos2, Rect, Shape as Drawn, Ui, epaint::Vertex, pos2};

/// How many points each shape has.
pub const POINTS: usize = 120;

/// The shapes M3E uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Circle,
    /// The Play button while paused.
    Squircle,
    /// Nine soft scallops: the Play button while playing, the brand.
    Cookie9,
    Cookie6,
    /// Eight shallow scallops: artists' pictures.
    Sunny,
    Clover4,
    Pentagon,
    Burst,
}

/// A cookie of `k` scallops `depth` deep.
fn cookie(k: f32, depth: f32, a: f32) -> f32 {
    0.5 * (1.0 - depth + depth * (0.5 + 0.5 * (k * a).cos()))
}

impl Shape {
    /// The radius at angle `a`, in a unit box (0.5 touches its edge).
    fn radius(self, a: f32) -> f32 {
        match self {
            Shape::Circle => 0.5,
            Shape::Squircle => 0.5 / (a.cos().abs().powi(4) + a.sin().abs().powi(4)).powf(0.25),
            Shape::Cookie9 => cookie(9.0, 0.13, a),
            Shape::Cookie6 => cookie(6.0, 0.16, a),
            Shape::Sunny => cookie(8.0, 0.09, a),
            Shape::Clover4 => 0.5 * (0.68 + 0.32 * (2.0 * a).cos().abs().powf(0.55)),
            Shape::Pentagon => cookie(5.0, 0.17, a),
            Shape::Burst => cookie(12.0, 0.12, a),
        }
    }

    /// Its radii, the first at the top, going clockwise.
    pub fn radii(self) -> Radii {
        Radii(std::array::from_fn(|i| self.radius(angle(i))))
    }
}

fn angle(i: usize) -> f32 {
    i as f32 / POINTS as f32 * TAU - FRAC_PI_2
}

/// A shape as its radii: what morphs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radii(pub [f32; POINTS]);

impl Radii {
    /// `t` of the way from `self` to `to`; past 1 or under 0 as a spring
    /// overshoots.
    pub fn toward(&self, to: &Radii, t: f32) -> Radii {
        Radii(std::array::from_fn(|i| {
            self.0[i] + (to.0[i] - self.0[i]) * t
        }))
    }

    /// The points round `rect`'s centre, the shape turned by `turn` radians.
    fn points(&self, rect: Rect, turn: f32) -> impl Iterator<Item = Pos2> + '_ {
        let c = rect.center();
        self.0.iter().enumerate().map(move |(i, r)| {
            let (sin, cos) = (angle(i) + turn).sin_cos();
            pos2(c.x + r * cos * rect.width(), c.y + r * sin * rect.height())
        })
    }

    /// The shape filled with `color` in `rect`.
    pub fn fill(&self, ui: &Ui, rect: Rect, turn: f32, color: Color32) {
        ui.painter()
            .add(Drawn::mesh(self.mesh(rect, turn, color, None)));
    }

    /// The image at `url` cut to the shape in `rect`; `fallback` until it is loaded.
    pub fn image(&self, ui: &Ui, url: &str, rect: Rect, turn: f32, fallback: Color32) {
        crate::covers::seen(ui.ctx(), url);
        let size = egui::load::SizeHint::Size {
            width: (rect.width() * ui.pixels_per_point()).round() as u32,
            height: (rect.height() * ui.pixels_per_point()).round() as u32,
            maintain_aspect_ratio: false,
        };
        match ui
            .ctx()
            .try_load_texture(url, egui::TextureOptions::LINEAR, size)
        {
            Ok(egui::load::TexturePoll::Ready { texture }) => {
                let mesh = self.mesh(rect, turn, Color32::WHITE, Some(texture.id));
                ui.painter().add(Drawn::mesh(mesh));
            }
            _ => self.fill(ui, rect, turn, fallback),
        }
    }

    fn mesh(
        &self,
        rect: Rect,
        turn: f32,
        color: Color32,
        texture: Option<egui::TextureId>,
    ) -> Mesh {
        let mut mesh = texture.map_or_else(Mesh::default, Mesh::with_texture);
        // A cover stays still while the shape turns over it.
        let uv = |p: Pos2| match texture {
            Some(_) => pos2(
                (p.x - rect.left()) / rect.width(),
                (p.y - rect.top()) / rect.height(),
            ),
            None => egui::epaint::WHITE_UV,
        };
        mesh.vertices.push(Vertex {
            pos: rect.center(),
            uv: uv(rect.center()),
            color,
        });
        for pos in self.points(rect, turn) {
            mesh.vertices.push(Vertex {
                pos,
                uv: uv(pos),
                color,
            });
        }
        for i in 0..POINTS as u32 {
            mesh.add_triangle(0, 1 + i, 1 + (i + 1) % POINTS as u32);
        }
        mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_fit_their_box_and_morph_end_to_end() {
        for shape in [
            Shape::Circle,
            Shape::Squircle,
            Shape::Cookie9,
            Shape::Cookie6,
            Shape::Sunny,
            Shape::Clover4,
            Shape::Pentagon,
            Shape::Burst,
        ] {
            let r = shape.radii();
            assert!(
                r.0.iter()
                    .all(|&r| r > 0.3 && r <= 0.5 * 2f32.sqrt() + 1e-4),
                "{shape:?}"
            );
        }
        let (a, b) = (Shape::Squircle.radii(), Shape::Cookie9.radii());
        assert_eq!(a.toward(&b, 0.0), a);
        assert_eq!(a.toward(&b, 1.0), b);
    }
}
