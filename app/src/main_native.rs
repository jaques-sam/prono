use eframe::AppCreator;
use log::error;
use prono::ReadConfig;

fn build_app<'a>(prono: impl prono_api::Surveys + 'static) -> AppCreator<'a> {
    Box::new(|cc: &eframe::CreationContext<'_>| Ok(Box::new(crate::App::new(cc, prono))))
}

/// # Panics
///
/// - if another used library has already initialized a global logger
/// - if the app icon cannot be loaded
///
/// # Errors
/// - fails if the graphics context cannot be created
#[tokio::main]
pub async fn main() -> eframe::Result {
    env_logger::init(); // Log to stdout iso stderr (if you run with e.g. `RUST_LOG=debug`).

    let config_reader = prono::factory::create_config_reader();
    let default_config_path = config_reader.default_config_path();
    let db_config: prono_db::Config = config_reader.read(default_config_path).db.into();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 400.0])
            .with_min_inner_size([300.0, 300.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    let prono = match prono::SyncPronoAdapter::new_with_db_config::<prono_db::MysqlDb>(db_config).await {
        Err(e) => {
            error!("{e}");
            return Ok(());
        }
        Ok(prono) => prono,
    };

    eframe::run_native("eframe template", native_options, build_app(prono))
}
