pub struct ViewportProjection;

impl ViewportProjection {
    /// Projects a 3D celestial coordinate (x, y, z) onto a 2D screen coordinate (sx, sy)
    pub fn project_3d_to_2d(
        x: f32,
        y: f32,
        z: f32,
        rot_x: f32,
        rot_y: f32,
        zoom: f32,
        screen_center: (f32, f32),
    ) -> (f32, f32, f32) {
        // Rotate around Y-axis
        let cos_y = rot_y.cos();
        let sin_y = rot_y.sin();
        let x1 = x * cos_y + z * sin_y;
        let z1 = -x * sin_y + z * cos_y;

        // Rotate around X-axis
        let cos_x = rot_x.cos();
        let sin_x = rot_x.sin();
        let y2 = y * cos_x - z1 * sin_x;
        let z2 = y * sin_x + z1 * cos_x;

        // Perspective division
        let distance = 350.0;
        let scale = (distance / (distance + z2)).max(0.2) * zoom;

        let screen_x = screen_center.0 + x1 * scale;
        let screen_y = screen_center.1 - y2 * scale; // Invert Y for screen coords

        (screen_x, screen_y, z2)
    }
}
