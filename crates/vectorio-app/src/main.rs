//! Vectorio · aplicación de escritorio.

rust_i18n::i18n!("locales", fallback = "es");

#[macro_use]
mod i18n;

mod app;
mod files;
mod recovery;
mod fmt;
mod canvas;
mod catalog;
mod colorpanel;
mod commands;
mod create;
mod inspector;
mod macos;
mod output;
mod paths;
mod prefs;
mod prefs_ui;
mod styles;
mod theme;
mod vectorize_ui;
mod volumetric_ui;

fn main() -> eframe::Result<()> {
    // En macOS eframe usa este icono en el Dock y en ⌘⇥ por encima del del paquete, así
    // que debe ser el de la retícula de Apple (placa de 824 px de 1024, centrada).
    #[cfg(target_os = "macos")]
    let png = include_bytes!("../../../assets/branding/vectorio-icon-macos.png");
    #[cfg(not(target_os = "macos"))]
    let png = include_bytes!("../../../assets/branding/vectorio-icon.png");
    let icon = image::load_from_memory(png)
        .expect("El icono de Vectorio debe ser un PNG válido").into_rgba8();
    let icon = egui::IconData { width: icon.width(), height: icon.height(), rgba: icon.into_raw() };
    #[cfg(target_os = "macos")]
    let viewport = egui::ViewportBuilder::default()
        .with_title("Vectorio")
        .with_icon(icon)
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([980.0, 620.0])
        // Barra de herramientas unificada: el contenido llega bajo los botones de la ventana.
        .with_fullsize_content_view(true)
        .with_title_shown(false)
        .with_titlebar_shown(false)
        .with_drag_and_drop(true);

    #[cfg(not(target_os = "macos"))]
    let viewport = egui::ViewportBuilder::default()
        .with_title("Vectorio")
        .with_icon(icon)
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([980.0, 620.0])
        .with_drag_and_drop(true);
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native("Vectorio", options, Box::new(|cc| Ok(Box::new(app::VectorioApp::new(cc)))))
}

