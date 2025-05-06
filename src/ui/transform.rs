// src/ui/transform.rs
use egui::{Pos2, Rect, Vec2};

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    // Pan (translation) in world coordinates.
    // Represents the world coordinate directly under the top-left corner (0,0) of the screen viewport.
    pub pan: Vec2,
    // Zoom factor. Higher value means zoomed in.
    pub zoom: f32,
    // The screen viewport rectangle (needed for some calculations, though not strictly part of the transform itself).
    // Note: Using screen_center as pivot was removed for simplicity, zoom is now relative to origin (0,0) screen.
    // pub screen_center: Pos2,
}

impl Transform {
    // Pass pan and zoom. screen_center is removed.
    pub fn new(pan: Vec2, zoom: f32 /*, screen_center: Pos2 */) -> Self {
        Self { pan, zoom /*, screen_center */ }
    }

    /// Converts screen coordinates to world coordinates.
    #[inline]
    pub fn screen_to_world(&self, screen_pos: Pos2) -> Pos2 {
        // World position = Pan + (Screen Position / Zoom Factor)
        // Addition Vec2 + Vec2 -> Vec2. Conversion en Pos2.
        Pos2::new(self.pan.x + screen_pos.x / self.zoom, self.pan.y + screen_pos.y / self.zoom)
    }

    /// Converts world coordinates to screen coordinates.
    #[inline]
    pub fn world_to_screen(&self, world_pos: Pos2) -> Pos2 {
        // Screen position = (World Position - Pan) * Zoom Factor
        // Soustraction Pos2 - Vec2 -> Pos2. Multiplication Pos2 * f32 n'existe pas.
        // Faire : (world_pos.to_vec2() - self.pan) * self.zoom -> Vec2, puis convertir en Pos2
        let screen_vec = (world_pos.to_vec2() - self.pan) * self.zoom;
        Pos2::new(screen_vec.x, screen_vec.y)
    }

    /// Converts world rectangle to screen rectangle.
    #[inline]
    pub fn world_rect_to_screen(&self, world_rect: Rect) -> Rect {
        Rect::from_min_max(
            self.world_to_screen(world_rect.min),
            self.world_to_screen(world_rect.max),
        )
    }

     /// Converts screen vector (like a mouse delta) to world vector.
     /// For vectors, only scaling applies, not panning.
     #[inline]
    pub fn screen_vec_to_world(&self, screen_vec: Vec2) -> Vec2 {
        screen_vec / self.zoom
    }

    /// Converts world vector to screen vector.
    /// For vectors, only scaling applies, not panning.
    #[inline]
    pub fn world_vec_to_screen(&self, world_vec: Vec2) -> Vec2 {
        world_vec * self.zoom
    }
}