use cynpase_bot::gui::CynpaseApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1050.0, 680.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("CynapseBot Desktop Control Panel"),
        ..Default::default()
    };

    eframe::run_native(
        "CynapseBot Control Panel",
        native_options,
        Box::new(|cc| Ok(Box::new(CynpaseApp::new(cc)))),
    )
}
