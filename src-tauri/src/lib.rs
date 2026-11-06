//! Biblioteca principal de Tauri v2 para la aplicación "Limpiador de CSV".
//!
//! Expone los comandos IPC invocados desde el frontend en JavaScript puro (`window.__TAURI__.core.invoke`):
//! - [`clean_csv_payload`]: Limpia y ordena un único archivo CSV en un clic.
//! - [`save_cleaned_csv`]: Abre el diálogo nativo de Windows "Guardar como..." sugiriendo `<nombre_original>_limpio.csv`
//!   y guarda el archivo resultante en UTF-8.

pub mod cleaner;

use cleaner::{process_csv_bytes, CsvCleanResponse, NullHandlingMode};
use std::fs;
use std::path::Path;

/// Resultado devuelto al guardar el archivo CSV limpio en disco.
#[derive(serde::Serialize)]
pub struct SaveCsvResult {
    /// Indica si el usuario completó el guardado (`true`) o canceló el diálogo (`false`).
    pub saved: bool,
    /// Ruta absoluta del archivo guardado en caso de éxito.
    pub file_path: Option<String>,
}

/// Comando Tauri que recibe el nombre del archivo, los bytes crudos del CSV cargado por Drag & Drop
/// o selector, el índice de la columna de ordenamiento y el modo de celdas vacías, y ejecuta la limpieza.
#[tauri::command]
fn clean_csv_payload(
    file_name: String,
    raw_bytes: Vec<u8>,
    sort_col: Option<usize>,
    null_mode: Option<NullHandlingMode>,
) -> Result<CsvCleanResponse, String> {
    if !file_name.to_ascii_lowercase().ends_with(".csv") {
        return Err("Solo se admiten archivos con extensión .csv (un archivo por vez).".to_string());
    }

    process_csv_bytes(
        &raw_bytes,
        &file_name,
        sort_col.unwrap_or(0),
        null_mode.unwrap_or_default(),
    )
}

/// Comando Tauri que abre el diálogo nativo "Guardar como..." de Windows sugiriendo
/// `<nombre_original>_limpio.csv` y escribe el contenido limpio en UTF-8.
#[tauri::command]
fn save_cleaned_csv(
    original_file_name: String,
    cleaned_csv_content: String,
) -> Result<SaveCsvResult, String> {
    let stem = Path::new(&original_file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("datos");

    let suggested_name = format!("{stem}_limpio.csv");

    let target_path = rfd::FileDialog::new()
        .set_title("Guardar archivo CSV limpio")
        .set_file_name(&suggested_name)
        .add_filter("Archivo CSV (*.csv)", &["csv"])
        .save_file();

    match target_path {
        Some(path) => {
            fs::write(&path, cleaned_csv_content.as_bytes())
                .map_err(|e| format!("No se pudo guardar el archivo en disco: {e}"))?;
            Ok(SaveCsvResult {
                saved: true,
                file_path: Some(path.display().to_string()),
            })
        }
        None => Ok(SaveCsvResult {
            saved: false,
            file_path: None,
        }),
    }
}

/// Punto de entrada principal del runtime Tauri v2.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            clean_csv_payload,
            save_cleaned_csv
        ])
        .run(tauri::generate_context!())
        .expect("Error al iniciar la aplicación Tauri Limpiador de CSV");
}

