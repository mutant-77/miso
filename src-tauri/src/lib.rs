mod clipboard;
mod commands;
mod doc_windows;
mod error;
mod export;
mod file_id;
mod fonts;
mod i18n;
mod import;
mod model;
mod positions;
mod prefs;
mod store;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::PoisonError;

use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

use doc_windows::{
    DocWindows, create_reader, focus, image_response, last_reader, open_doc, open_paths,
};
use error::{AppError, report};
use positions::PositionsState;
use prefs::PrefsState;

pub fn run() {
    let mut log = tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::LogDir { file_name: None }))
        .max_file_size(1_000_000)
        .rotation_strategy(RotationStrategy::KeepOne)
        .level(log::LevelFilter::Info);
    if cfg!(debug_assertions) {
        log = log.target(Target::new(TargetKind::Stdout));
    }

    tauri::Builder::default()
        .register_uri_scheme_protocol("miso-img", |ctx, request| {
            let state = ctx.app_handle().state::<DocWindows>();
            let slots = state.slots.lock().unwrap_or_else(PoisonError::into_inner);
            image_response(&slots, ctx.webview_label(), request.uri().path())
        })
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let paths = rutas(argv.into_iter().skip(1), Path::new(&cwd));
            if !paths.is_empty() {
                tauri::async_runtime::spawn(open_paths(
                    app.clone(),
                    last_reader(app).unwrap_or_default(),
                    paths,
                ));
            } else if let Some(label) = last_reader(app)
                && let Err(err) = focus(app, &label)
            {
                log::warn!("no se pudo enfocar {label}: {err}");
            }
        }))
        .plugin(log.build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::take_pending_document,
            commands::set_position,
            commands::open_from_clipboard,
            commands::open_dialog,
            commands::save_copy,
            commands::get_prefs,
            commands::set_prefs,
            commands::reset_prefs,
            commands::list_fonts
        ])
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(true) if window.label().starts_with("reader-") => {
                let state = window.state::<DocWindows>();
                *state.focused.lock().unwrap_or_else(PoisonError::into_inner) =
                    window.label().to_owned();
            }
            WindowEvent::Destroyed => {
                window.state::<DocWindows>().slots.lock().unwrap().close(window.label());
                if window.label().starts_with("reader-") {
                    window.state::<PositionsState>().flush();
                }
            }
            WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                tauri::async_runtime::spawn(open_paths(
                    window.app_handle().clone(),
                    window.label().to_owned(),
                    paths.clone(),
                ));
            }
            _ => {}
        })
        .setup(|app| {
            log::info!("Miso {} iniciado", app.package_info().version);
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                log::error!("pánico: {info}");
                previous(info);
            }));
            app.manage(DocWindows::new());
            let prefs = PrefsState::load(app.path().app_data_dir()?.join("prefs.json"));
            i18n::set(i18n::effective(&prefs.prefs.lock().unwrap().language));
            app.manage(prefs);
            app.manage(PositionsState::load(app.path().app_data_dir()?.join("positions.json")));
            ventana_inicial(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error al iniciar Miso")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<PositionsState>().flush();
            }
        });
}

fn rutas<I, S>(args: I, cwd: &Path) -> Vec<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    args.into_iter()
        .map(Into::into)
        .filter(|arg: &OsString| !arg.to_string_lossy().starts_with('-'))
        .map(|arg| cwd.join(arg))
        .collect()
}

fn ventana_inicial(app: &AppHandle) -> Result<(), AppError> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let paths = rutas(std::env::args_os().skip(1), &cwd);
    let con_archivos = !paths.is_empty();
    if con_archivos {
        // Sin ventana de origen, open_doc crea la ventana ya con el libro dentro.
        tauri::async_runtime::block_on(open_paths(app.clone(), "reader-0".into(), paths));
        if !app.webview_windows().is_empty() {
            return Ok(());
        }
    }
    let window = create_reader(app, "reader-0", "Miso")?;
    if !con_archivos
        && let Some(doc) = clipboard::read_doc()
        && let Err(err) = open_doc(app, "reader-0", doc, None)
    {
        report(app, Some(&window), &err, "portapapeles");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::rutas;
    use std::path::{Path, PathBuf};

    const CWD: &str = "C:\\dir";

    #[test]
    fn relativa_se_une_al_cwd() {
        let paths = rutas(["libro.txt"], Path::new(CWD));
        assert_eq!(paths, vec![PathBuf::from("C:\\dir\\libro.txt")]);
    }

    #[test]
    fn absoluta_se_conserva() {
        let paths = rutas(["D:\\otro\\libro.txt"], Path::new(CWD));
        assert_eq!(paths, vec![PathBuf::from("D:\\otro\\libro.txt")]);
    }

    #[test]
    fn flags_se_descartan() {
        let paths = rutas(["-x", "--y", "libro.txt"], Path::new(CWD));
        assert_eq!(paths, vec![PathBuf::from("C:\\dir\\libro.txt")]);
    }

    #[test]
    fn sin_argumentos_no_hay_rutas() {
        let paths = rutas(std::iter::empty::<&str>(), Path::new(CWD));
        assert!(paths.is_empty());
    }
}
