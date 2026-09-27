use crate::gui::projection::ViewportProjection;
use crate::gui::state::GuiState;
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

pub struct CynpaseApp {
    pub state: GuiState,
}

impl CynpaseApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: GuiState::default(),
        }
    }
}

impl eframe::App for CynpaseApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top Header
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🤖 CynapseBot Desktop Control Panel");
                ui.separator();
                let status_color = if self.state.connected { Color32::GREEN } else { Color32::RED };
                ui.colored_label(status_color, if self.state.connected { "● ONLINE" } else { "○ DISCONNECTED" });
                ui.label(format!("Hub: {}", self.state.hub_url));
            });
        });

        // Left Panel: Robot & Hardware Controls
        egui::SidePanel::left("controls").min_width(280.0).show(ctx, |ui| {
            ui.heading("Hardware Actuation");
            ui.add_space(8.0);

            ui.group(|ui| {
                ui.label("Pan / Tilt Servo Control");
                ui.add(egui::Slider::new(&mut self.state.pan, -90..=90).text("Pan (Yaw)"));
                ui.add(egui::Slider::new(&mut self.state.tilt, -30..=30).text("Tilt (Pitch)"));

                if ui.button("Center Head (0°, 0°)").clicked() {
                    self.state.pan = 0;
                    self.state.tilt = 0;
                }
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label("Choreographed Animations");
                ui.horizontal_wrapped(|ui| {
                    if ui.button("💃 Dance").clicked() {
                        self.state.add_transcript("GUI", "Triggered Dance animation");
                    }
                    if ui.button("👋 Wave").clicked() {
                        self.state.add_transcript("GUI", "Triggered Wave animation");
                    }
                    if ui.button("🙂 Nod").clicked() {
                        self.state.add_transcript("GUI", "Triggered Nod animation");
                    }
                    if ui.button("↔ Shake").clicked() {
                        self.state.add_transcript("GUI", "Triggered Shake animation");
                    }
                    if ui.button("💤 Sleep").clicked() {
                        self.state.add_transcript("GUI", "Triggered Sleep animation");
                    }
                    if ui.button("☀️ Wake").clicked() {
                        self.state.add_transcript("GUI", "Triggered Wake animation");
                    }
                });
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label("Onboard Neon LED Color");
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(&mut [self.state.neon_red, self.state.neon_green, self.state.neon_blue]);
                    ui.label(format!("RGB({}, {}, {})", self.state.neon_red, self.state.neon_green, self.state.neon_blue));
                });
            });

            ui.add_space(8.0);
            if ui.button("🚨 EMERGENCY STOP").clicked() {
                self.state.add_transcript("SAFETY", "Emergency stop triggered");
            }
        });

        // Central Panel: Mazzaroth Constellation Galaxy View & Camera
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                // Column 1: Live Feed & Transcript
                columns[0].vertical(|ui| {
                    ui.heading("Camera & Voice Feed");
                    
                    // Camera Placeholder Box
                    let (rect, _response) = ui.allocate_exact_size(Vec2::new(320.0, 240.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 8.0, Color32::from_rgb(15, 23, 42));
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "📷 Camera Stream (0.3MP GC0308)\nReady for Robot Feed",
                        egui::FontId::monospace(14.0),
                        Color32::from_rgb(148, 163, 184),
                    );

                    ui.add_space(10.0);
                    ui.heading("Transcript Logs");
                    egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                        for (ts, msg) in &self.state.transcripts {
                            ui.label(format!("[{}] {}", ts, msg));
                        }
                    });
                });

                // Column 2: Mazzaroth 3D Celestial Galaxy Canvas
                columns[1].vertical(|ui| {
                    ui.heading("🌌 Mazzaroth Memory Galaxy");
                    ui.label("Interactive 3D Constellation View (Drag to rotate)");

                    let (rect, response) = ui.allocate_exact_size(Vec2::new(400.0, 380.0), egui::Sense::drag());
                    
                    if response.dragged() {
                        let delta = response.drag_delta();
                        self.state.rot_y += delta.x * 0.01;
                        self.state.rot_x -= delta.y * 0.01;
                    }

                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 8.0, Color32::from_rgb(11, 15, 25));

                    let center = (rect.center().x, rect.center().y);

                    // Render central galactic hub
                    let (cx, cy, _) = ViewportProjection::project_3d_to_2d(0.0, 0.0, 0.0, self.state.rot_x, self.state.rot_y, self.state.zoom, center);
                    painter.circle_filled(Pos2::new(cx, cy), 10.0, Color32::from_rgb(250, 204, 21)); // Gold central star
                    painter.text(Pos2::new(cx, cy + 14.0), egui::Align2::CENTER_TOP, "Identity Core", egui::FontId::proportional(11.0), Color32::WHITE);

                    // Render sample planetary nodes
                    let sample_stars = [
                        (80.0, 50.0, 20.0, "User: Xander", Color32::from_rgb(56, 189, 248)),
                        (-90.0, 70.0, -30.0, "Skill: Rust", Color32::from_rgb(74, 222, 128)),
                        (40.0, -80.0, 60.0, "Project: Cynapse", Color32::from_rgb(192, 132, 252)),
                        (-60.0, -60.0, -50.0, "Rule: Local-First", Color32::from_rgb(251, 146, 60)),
                    ];

                    for (x, y, z, label, color) in sample_stars {
                        let (px, py, _) = ViewportProjection::project_3d_to_2d(x, y, z, self.state.rot_x, self.state.rot_y, self.state.zoom, center);
                        // Draw constellation line to center
                        painter.line_segment([Pos2::new(cx, cy), Pos2::new(px, py)], Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(100, 116, 139, 100)));
                        // Draw star
                        painter.circle_filled(Pos2::new(px, py), 6.0, color);
                        painter.text(Pos2::new(px, py + 8.0), egui::Align2::CENTER_TOP, label, egui::FontId::proportional(10.0), Color32::from_rgb(226, 232, 240));
                    }
                });
            });
        });
    }
}
