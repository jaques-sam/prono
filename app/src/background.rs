//! The page background of kr8ium.com: anthracite that takes a pine tint
//! towards the middle, a faint dot grid and a soft green glow.

use egui::{Color32, Mesh, Painter, Rect, Shape, pos2, vec2};

const ANTHRACITE: Color32 = Color32::from_rgb(0x11, 0x18, 0x27);
/// `pine` is `#1F4D2E`; `alpha` is the opacity out of 255.
pub fn pine(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(0x1f, 0x4d, 0x2e, alpha)
}

/// `via-pine/15`: the tint of the vertical gradient in the middle.
const TINT_ALPHA: u8 = 38;
const GRID_ALPHA: u8 = 204;
const GLOW_ALPHA: u8 = 128;
/// The distance between the dots of the grid.
const GRID_STEP: f32 = 32.0;
/// Where the glow is centered, and how far it reaches, relative to the size.
const GLOW_CENTER: (f32, f32) = (0.6, 0.4);
const GLOW_REACH: f32 = 0.7;
const GLOW_SEGMENTS: u32 = 48;

/// Paints the background over `rect`, within the clip rectangle of `painter`.
pub fn paint(painter: &Painter, rect: Rect) {
    painter.rect_filled(rect, 0.0, ANTHRACITE);
    painter.add(Shape::mesh(tint(rect)));
    painter.extend(dots(rect));
    painter.add(Shape::mesh(glow(rect)));
}

/// Transparent at the top and bottom, tinted in the middle.
fn tint(rect: Rect) -> Mesh {
    let mut mesh = Mesh::default();
    for (y, color) in [
        (rect.top(), Color32::TRANSPARENT),
        (rect.center().y, pine(TINT_ALPHA)),
        (rect.bottom(), Color32::TRANSPARENT),
    ] {
        mesh.colored_vertex(pos2(rect.left(), y), color);
        mesh.colored_vertex(pos2(rect.right(), y), color);
    }
    for band in 0..2 {
        let first = band * 2;
        mesh.add_triangle(first, first + 1, first + 2);
        mesh.add_triangle(first + 1, first + 2, first + 3);
    }
    mesh
}

fn dots(rect: Rect) -> Vec<Shape> {
    let color = pine(GRID_ALPHA);
    let mut dots = Vec::new();
    let mut y = rect.top() + 1.0;
    while y < rect.bottom() {
        let mut x = rect.left() + 1.0;
        while x < rect.right() {
            dots.push(Shape::circle_filled(pos2(x, y), 1.0, color));
            x += GRID_STEP;
        }
        y += GRID_STEP;
    }
    dots
}

/// An ellipse that fades from the center to nothing, as a triangle fan.
#[allow(clippy::cast_precision_loss)]
fn glow(rect: Rect) -> Mesh {
    let center = rect.min + vec2(rect.width() * GLOW_CENTER.0, rect.height() * GLOW_CENTER.1);
    let reach = rect.size() * GLOW_REACH;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, pine(GLOW_ALPHA));
    for segment in 0..GLOW_SEGMENTS {
        let angle = std::f32::consts::TAU * segment as f32 / GLOW_SEGMENTS as f32;
        let edge = center + vec2(angle.cos() * reach.x, angle.sin() * reach.y);
        mesh.colored_vertex(edge, Color32::TRANSPARENT);
    }
    for segment in 0..GLOW_SEGMENTS {
        mesh.add_triangle(0, 1 + segment, 1 + (segment + 1) % GLOW_SEGMENTS);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 64.0))
    }

    #[test]
    fn meshes_are_valid_and_stay_within_their_indices() {
        for mesh in [tint(rect()), glow(rect())] {
            assert!(mesh.is_valid());
            assert!(!mesh.is_empty());
        }
    }

    #[test]
    fn tint_is_only_visible_in_the_middle() {
        let alphas: Vec<_> = tint(rect()).vertices.iter().map(|vertex| vertex.color.a()).collect();

        assert_eq!(alphas, [0, 0, TINT_ALPHA, TINT_ALPHA, 0, 0]);
    }

    #[test]
    fn the_dots_form_a_grid_inside_the_rectangle() {
        let dots = dots(rect());

        // 4 columns (x = 11, 43, 75, 107) by 2 rows (y = 21, 53).
        assert_eq!(dots.len(), 4 * 2);
        assert!(dots.iter().all(|dot| rect().contains_rect(dot.visual_bounding_rect())));
    }

    #[test]
    fn glow_is_strongest_at_its_center() {
        let mesh = glow(rect());

        assert_eq!(mesh.vertices[0].color, pine(GLOW_ALPHA));
        assert!(
            mesh.vertices[1..]
                .iter()
                .all(|vertex| vertex.color == Color32::TRANSPARENT)
        );
    }
}
