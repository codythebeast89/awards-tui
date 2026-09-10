//! `awards-gui` entry point: a native desktop window offering the first GUI milestone's two
//! capabilities — Lookup and Add (spec 004-gui-lookup-add) — for the FORSCOM Decorations
//! Database. Presentation-only: every byte of parsing, matching, and Sheets I/O is reused
//! unchanged from `awards-core`/`awards-sheets`, exactly as `awards-tui` already does.

mod app;
mod ui;

use app::GuiApp;
use eframe::egui;

struct Application {
    inner: GuiApp,
}

impl eframe::App for Application {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.inner.drain_messages();
        ui::render(&mut self.inner, ui);
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "QMC Decoration Database",
        options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();
            let inner = GuiApp::new(move || ctx.request_repaint());
            Ok(Box::new(Application { inner }))
        }),
    )
}
