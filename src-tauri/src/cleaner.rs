//! Motor de análisis, limpieza y ordenamiento de archivos CSV.
//!
//! Este módulo implementa las 5 reglas de limpieza solicitadas:
//! 1. Eliminación de espacios extra (`trim` y colapso de espacios internos múltiples) en cabeceras y celdas.
//! 2. Eliminación de filas completamente vacías y filas duplicadas exactas.
//! 3. Normalización automática de separadores (`,`, `;`, `\t`, `|`) a coma estándar `,` y codificación UTF-8.
//! 4. Tratamiento de celdas vacías / valores nulos (`NaN`, `null`, `N/A`, vacíos) permitiendo rellenar o descartar filas incompletas.
//! 5. Estandarización de nombres de columnas a formato `snake_case` sin tildes ni caracteres especiales.
//!
//! Además, ordena las filas ascendentemente por la columna seleccionada (por defecto la columna 0),
//! comparando de forma inteligente valores numéricos y alfanuméricos, y devuelve las 5 primeras filas
//! del estado original (desordenado/sucio) y del estado limpio (ordenado) para la visualización doble.

use encoding_rs::{UTF_8, WINDOWS_1252};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

/// Modo de tratamiento para filas con celdas vacías o valores nulos (`NaN`, `NULL`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NullHandlingMode {
    /// Rellena las celdas vacías o nulas con un valor estándar (`"N/D"`).
    FillDefault,
    /// Elimina cualquier fila que contenga al menos una celda vacía o nula.
    DropRow,
}

impl Default for NullHandlingMode {
    fn default() -> Self {
        Self::FillDefault
    }
}

/// Estadísticas detalladas del proceso de limpieza en un solo clic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleaningStats {
    /// Delimitador original detectado automáticamente (ej. `,`, `;`, `\t`, `|`).
    pub detected_delimiter: String,
    /// Codificación original detectada (`UTF-8` o `Windows-1252 / Latin-1`).
    pub detected_encoding: String,
    /// Total de filas de datos leídas en el archivo original (sin contar cabecera).
    pub original_row_count: usize,
    /// Total de filas resultantes tras la limpieza.
    pub cleaned_row_count: usize,
    /// Cantidad de filas completamente vacías eliminadas.
    pub empty_rows_removed: usize,
    /// Cantidad de filas duplicadas exactas eliminadas.
    pub duplicate_rows_removed: usize,
    /// Cantidad de celdas vacías/nulas rellenadas o filas con nulos descartadas.
    pub null_cells_handled: usize,
    /// Cantidad de celdas cuyos espacios fueron recortados/normalizados.
    pub trimmed_cells_count: usize,
}

/// Resultado completo del procesamiento de un archivo CSV para alimentar la interfaz gráfica.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvCleanResponse {
    /// Nombre del archivo original cargado.
    pub file_name: String,
    /// Cabeceras originales tal como venían en el archivo.
    pub original_headers: Vec<String>,
    /// Cabeceras estandarizadas en formato `snake_case` sin tildes.
    pub cleaned_headers: Vec<String>,
    /// Primeras 5 filas del archivo original (desordenadas y sin limpiar) para el panel izquierdo.
    pub original_preview_rows: Vec<Vec<String>>,
    /// Primeras 5 filas del archivo limpio y ordenado para el panel derecho.
    pub cleaned_preview_rows: Vec<Vec<String>>,
    /// Índice de la columna utilizada para ordenar (0 por defecto).
    pub sort_column_index: usize,
    /// Contenido completo del CSV limpio y ordenado serializado con `,` y UTF-8, listo para exportar.
    pub cleaned_csv_content: String,
    /// Resumen estadístico de las operaciones aplicadas.
    pub stats: CleaningStats,
}

/// Decodifica bytes crudos garantizando salida UTF-8 válida.
/// Si el archivo tiene BOM UTF-8 o es UTF-8 válido, lo decodifica directamente.
/// En caso contrario, aplica fallback a `Windows-1252` (ANSI/Latin-1 común en Excel en español).
pub fn decode_to_utf8(bytes: &[u8]) -> (String, String) {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let (cow, _, _) = UTF_8.decode(&bytes[3..]);
        return (cow.into_owned(), "UTF-8 (BOM)".to_string());
    }
    match std::str::from_utf8(bytes) {
        Ok(valid_utf8) => (valid_utf8.to_string(), "UTF-8".to_string()),
        Err(_) => {
            let (cow, _, _) = WINDOWS_1252.decode(bytes);
            (cow.into_owned(), "Windows-1252 / Latin-1 → UTF-8".to_string())
        }
    }
}

/// Detecta automáticamente el delimitador más probable analizando las primeras líneas no vacías.
pub fn detect_delimiter(text: &str) -> u8 {
    let candidates: [u8; 4] = [b',', b';', b'\t', b'|'];
    let sample_lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(10)
        .collect();

    if sample_lines.is_empty() {
        return b',';
    }

    let mut best_delim = b',';
    let mut best_score: usize = 0;

    for &delim in &candidates {
        let ch = delim as char;
        let counts: Vec<usize> = sample_lines
            .iter()
            .map(|line| line.chars().filter(|&c| c == ch).count())
            .collect();

        let total: usize = counts.iter().sum();
        if total == 0 {
            continue;
        }
        // Bonificamos consistencia entre líneas y frecuencia total
        let non_zero_lines = counts.iter().filter(|&&c| c > 0).count();
        let score = total * 10 + non_zero_lines * 25;
        if score > best_score {
            best_score = score;
            best_delim = delim;
        }
    }

    best_delim
}

/// Convierte cualquier nombre de columna a `snake_case` limpio y sin tildes (NFD Unicode decomposition).
/// Ejemplo: `"  Año de Ingreso (%) "` -> `"ano_de_ingreso"`
pub fn to_snake_case_header(input: &str, col_index: usize) -> String {
    // 1. Descomponer caracteres Unicode (NFD) y descartar marcas diacríticas (tildes, diéresis)
    //    Además tratamos ñ/Ñ explícitamente como n.
    let deaccented: String = input
        .trim()
        .nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect();

    let mut result = String::with_capacity(deaccented.len());
    let mut prev_was_underscore = false;
    let mut prev_was_lower_or_digit = false;

    for ch in deaccented.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if prev_was_lower_or_digit && !prev_was_underscore && !result.is_empty() {
                    result.push('_');
                }
                result.push(ch.to_ascii_lowercase());
                prev_was_lower_or_digit = false;
            } else {
                result.push(ch);
                prev_was_lower_or_digit = true;
            }
            prev_was_underscore = false;
        } else if !result.is_empty() && !prev_was_underscore {
            result.push('_');
            prev_was_underscore = true;
            prev_was_lower_or_digit = false;
        }
    }

    let trimmed = result.trim_matches('_').to_string();
    if trimmed.is_empty() {
        format!("columna_{}", col_index + 1)
    } else {
        trimmed
    }
}

/// Limpia los espacios de una celda: recorta extremos (`trim`) y colapsa múltiples espacios internos.
pub fn clean_cell_whitespace(value: &str) -> (String, bool) {
    let trimmed = value.trim();
    let mut collapsed = String::with_capacity(trimmed.len());
    let mut prev_space = false;

    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                collapsed.push(' ');
                prev_space = true;
            }
        } else {
            collapsed.push(ch);
            prev_space = false;
        }
    }

    let changed = collapsed != value;
    (collapsed, changed)
}

/// Determina si el contenido de una celda (ya recortada) representa un valor nulo o vacío.
pub fn is_null_like(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    matches!(
        value.to_ascii_lowercase().as_str(),
        "nan" | "null" | "none" | "n/a" | "na" | "#n/a" | "nil" | "-"
    )
}

/// Compara dos celdas priorizando orden numérico real (si ambos valores son números válidos)
/// y utilizando orden alfabético insensible a mayúsculas como respaldo.
pub fn compare_cells(a: &str, b: &str) -> Ordering {
    let parse_num = |s: &str| -> Option<f64> {
        let normalized = s.replace(',', ".");
        normalized.parse::<f64>().ok()
    };

    match (parse_num(a), parse_num(b)) {
        (Some(na), Some(nb)) => na.partial_cmp(&nb).unwrap_or(Ordering::Equal),
        _ => a
            .to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b)),
    }
}

/// Procesa, limpia y ordena un archivo CSV completo en memoria a partir de sus bytes crudos.
pub fn process_csv_bytes(
    bytes: &[u8],
    file_name: &str,
    sort_col: usize,
    null_mode: NullHandlingMode,
) -> Result<CsvCleanResponse, String> {
    let (decoded_text, detected_encoding) = decode_to_utf8(bytes);
    if decoded_text.trim().is_empty() {
        return Err("El archivo CSV está vacío o no contiene datos legibles.".to_string());
    }

    let delimiter = detect_delimiter(&decoded_text);
    let delimiter_label = match delimiter {
        b',' => "Coma (,)".to_string(),
        b';' => "Punto y coma (;)".to_string(),
        b'\t' => "Tabulación (\\t)".to_string(),
        b'|' => "Barra vertical (|)".to_string(),
        other => format!("Carácter ({})", other as char),
    };

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .has_headers(true)
        .from_reader(decoded_text.as_bytes());

    let raw_headers_record = reader
        .headers()
        .map_err(|e| format!("No se pudieron leer las cabeceras del CSV: {e}"))?
        .clone();

    if raw_headers_record.is_empty() {
        return Err("El archivo CSV no contiene columnas válidas en su cabecera.".to_string());
    }

    let original_headers: Vec<String> = raw_headers_record.iter().map(|h| h.to_string()).collect();
    let num_cols = original_headers.len();

    // Regla 1 y 5 en cabeceras: trim + estandarización a snake_case sin tildes y desambiguación de duplicados
    let mut cleaned_headers: Vec<String> = Vec::with_capacity(num_cols);
    let mut used_headers: HashSet<String> = HashSet::new();
    for (idx, raw_h) in original_headers.iter().enumerate() {
        let base = to_snake_case_header(raw_h, idx);
        let mut candidate = base.clone();
        let mut suffix = 2;
        while used_headers.contains(&candidate) {
            candidate = format!("{base}_{suffix}");
            suffix += 1;
        }
        used_headers.insert(candidate.clone());
        cleaned_headers.push(candidate);
    }

    let mut original_rows: Vec<Vec<String>> = Vec::new();
    let mut cleaned_rows: Vec<Vec<String>> = Vec::new();
    let mut seen_rows: HashSet<Vec<String>> = HashSet::new();

    let mut empty_rows_removed = 0usize;
    let mut duplicate_rows_removed = 0usize;
    let mut null_cells_handled = 0usize;
    let mut trimmed_cells_count = 0usize;

    for result in reader.records() {
        let record = result.map_err(|e| format!("Error leyendo registro CSV: {e}"))?;

        // Normalizamos longitud de fila al número de columnas de la cabecera
        let mut raw_row: Vec<String> = (0..num_cols)
            .map(|i| record.get(i).unwrap_or("").to_string())
            .collect();

        // Guardamos en original_rows exactamente como vino (incluso con espacios o vacíos)
        original_rows.push(raw_row.clone());

        // Regla 1: Eliminar espacios extra en cada celda
        let mut trimmed_row: Vec<String> = Vec::with_capacity(num_cols);
        for cell in raw_row.drain(..) {
            let (cleaned_cell, was_trimmed) = clean_cell_whitespace(&cell);
            if was_trimmed {
                trimmed_cells_count += 1;
            }
            trimmed_row.push(cleaned_cell);
        }

        // Regla 2a: Eliminar filas completamente vacías (todas las celdas vacías)
        if trimmed_row.iter().all(|c| c.is_empty()) {
            empty_rows_removed += 1;
            continue;
        }

        // Regla 2b: Eliminar filas duplicadas exactas (antes de contabilizar nulos para no contarlos dos veces)
        if !seen_rows.insert(trimmed_row.clone()) {
            duplicate_rows_removed += 1;
            continue;
        }

        // Regla 4: Rellenar o eliminar filas con celdas vacías / valores nulos (NaN, null, etc.)
        let null_count_in_row = trimmed_row.iter().filter(|c| is_null_like(c)).count();
        if null_count_in_row > 0 {
            match null_mode {
                NullHandlingMode::DropRow => {
                    null_cells_handled += null_count_in_row;
                    continue;
                }
                NullHandlingMode::FillDefault => {
                    for cell in &mut trimmed_row {
                        if is_null_like(cell) {
                            *cell = "N/D".to_string();
                            null_cells_handled += 1;
                        }
                    }
                }
            }
        }

        cleaned_rows.push(trimmed_row);
    }

    let effective_sort_col = if sort_col < num_cols { sort_col } else { 0 };

    // Ordenar las filas limpias por la columna seleccionada (ascendente)
    cleaned_rows.sort_by(|row_a, row_b| {
        let cell_a = row_a.get(effective_sort_col).map(|s| s.as_str()).unwrap_or("");
        let cell_b = row_b.get(effective_sort_col).map(|s| s.as_str()).unwrap_or("");
        compare_cells(cell_a, cell_b)
    });

    // Extraer las 5 primeras filas de cada lado para la visualización doble
    let original_preview_rows: Vec<Vec<String>> = original_rows.iter().take(5).cloned().collect();
    let cleaned_preview_rows: Vec<Vec<String>> = cleaned_rows.iter().take(5).cloned().collect();

    // Regla 3: Serializar con separador coma (`,`) estándar y codificación UTF-8
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b',')
        .from_writer(vec![]);

    writer
        .write_record(&cleaned_headers)
        .map_err(|e| format!("Error al escribir cabeceras limpias: {e}"))?;

    for row in &cleaned_rows {
        writer
            .write_record(row)
            .map_err(|e| format!("Error al escribir fila limpia: {e}"))?;
    }

    let csv_bytes = writer
        .into_inner()
        .map_err(|e| format!("Error al finalizar generación del CSV limpio: {e}"))?;
    let cleaned_csv_content =
        String::from_utf8(csv_bytes).map_err(|e| format!("Error UTF-8 al generar salida: {e}"))?;

    let stats = CleaningStats {
        detected_delimiter: delimiter_label,
        detected_encoding,
        original_row_count: original_rows.len(),
        cleaned_row_count: cleaned_rows.len(),
        empty_rows_removed,
        duplicate_rows_removed,
        null_cells_handled,
        trimmed_cells_count,
    };

    Ok(CsvCleanResponse {
        file_name: file_name.to_string(),
        original_headers,
        cleaned_headers,
        original_preview_rows,
        cleaned_preview_rows,
        sort_column_index: effective_sort_col,
        cleaned_csv_content,
        stats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snake_case_headers_strip_accents_and_symbols() {
        assert_eq!(to_snake_case_header("  ID Cliente  ", 0), "id_cliente");
        assert_eq!(
            to_snake_case_header("Año de Facturación ($)", 1),
            "ano_de_facturacion"
        );
        assert_eq!(to_snake_case_header("NombreCompleto", 2), "nombre_completo");
        assert_eq!(to_snake_case_header("   ", 3), "columna_4");
    }

    #[test]
    fn test_full_cleaning_and_sorting_pipeline() {
        let raw_csv = "  ID ; Nombre Cliente ; Ciudad ; Monto \n\
                       30 ;   Ana Gómez   ; Córdoba ; 1500.50 \n\
                       ; ; ; \n\
                       10 ; Bruno Díaz ; Buenos Aires ; NaN \n\
                       10 ; Bruno Díaz ; Buenos Aires ; NaN \n\
                       20 ; Carla Pérez ; Mendoza ; 890.00 \n\
                       5 ; Diego Torres ; Rosario ; 2100.00 \n\
                       40 ; Elena Paz ; Salta ; 450.25 \n\
                       50 ; Fabián Ruiz ; Tucumán ; 300.00 \n";

        let res = process_csv_bytes(
            raw_csv.as_bytes(),
            "clientes_sucios.csv",
            0,
            NullHandlingMode::FillDefault,
        )
        .expect("El procesamiento debe ser exitoso");

        assert_eq!(
            res.cleaned_headers,
            vec!["id", "nombre_cliente", "ciudad", "monto"]
        );
        assert_eq!(res.stats.empty_rows_removed, 1);
        assert_eq!(res.stats.duplicate_rows_removed, 1);
        assert_eq!(res.stats.null_cells_handled, 1);
        assert_eq!(res.stats.cleaned_row_count, 6);

        // Las vistas previas solo muestran hasta las 5 primeras filas
        assert_eq!(res.original_preview_rows.len(), 5);
        assert_eq!(res.cleaned_preview_rows.len(), 5);

        // Verificar que el lado izquierdo preserva las filas desordenadas originales (primera fila: ID 30)
        assert_eq!(res.original_preview_rows[0][0].trim(), "30");

        // Verificar que el lado derecho tiene las filas ordenadas numéricamente por ID: 5, 10, 20, 30, 40
        let sorted_ids: Vec<&str> = res
            .cleaned_preview_rows
            .iter()
            .map(|r| r[0].as_str())
            .collect();
        assert_eq!(sorted_ids, vec!["5", "10", "20", "30", "40"]);

        // Verificar que el valor NaN fue rellenado con "N/D"
        assert_eq!(res.cleaned_preview_rows[1][3], "N/D");
    }

    #[test]
    fn test_drop_row_null_handling_mode() {
        let raw_csv = "codigo,producto\n2,Teclado\n1,NaN\n3,Mouse\n";
        let res = process_csv_bytes(
            raw_csv.as_bytes(),
            "productos.csv",
            0,
            NullHandlingMode::DropRow,
        )
        .expect("Debe limpiar y descartar fila con NaN");

        assert_eq!(res.stats.cleaned_row_count, 2);
        assert_eq!(res.cleaned_preview_rows[0][0], "2");
        assert_eq!(res.cleaned_preview_rows[1][0], "3");
    }
}

