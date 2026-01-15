use eframe::AppCreator;
use prono::ReadConfig;
use std::path::Path;

static CONFIG_FILENAME: &str = "secure_config.toml";
pub static DB_NAME: &str = "db_prono";

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
pub fn main() -> eframe::Result {
    env_logger::init(); // Log to stdout iso stderr (if you run with e.g. `RUST_LOG=debug`).

    let db_config: prono_db::Config = crate::ConfigRead {}.read(Path::new(CONFIG_FILENAME)).db.into();

    let db_future = async move {
        let db = prono_db::MysqlDb::connect_async(&db_config).await.expect("connect");
        Box::new(db) as Box<dyn prono::repo::Db + Send + Sync>
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    let prono = prono::SyncPronoAdapter::new_with_async_api_future(db_future);

    eframe::run_native("eframe template", native_options, build_app(prono))
}
