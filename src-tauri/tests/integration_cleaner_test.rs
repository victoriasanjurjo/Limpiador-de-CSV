//! Pruebas de integración para el motor de limpieza y ordenamiento de Limpiador de CSV.
//!
//! Valida el comportamiento de extremo a extremo de las 5 reglas de limpieza
//! y la generación de las vistas previas de 5 filas (desordenadas vs ordenadas).

use limpiador_de_csv_lib::cleaner::{process_csv_bytes, NullHandlingMode};

#[test]
fn integration_test_five_rules_and_dual_five_row_preview() {
    let dirty_csv = "\
  Código Empleado ; Nombre Completo ; Área de Trabajo ; Salario Mensual ($) \n\
  500 ;   Lucía   Fernández  ; Ingeniería ; 3200 \n\
  ; ; ; \n\
  100 ; Juan   Pérez ; Finanzas ; 2500 \n\
  100 ; Juan Pérez ; Finanzas ; 2500 \n\
  400 ; Ana   Silva ; Diseño ; NaN \n\
  200 ; Carlos Gómez ; Operaciones ; 2100 \n\
  300 ; Beatriz López ; Marketing ; 2800 \n\
  600 ; Nicolás Castro ; Ventas ; 1950 \n";

    let response = process_csv_bytes(
        dirty_csv.as_bytes(),
        "empleados_sucios.csv",
        0,
        NullHandlingMode::FillDefault,
    )
    .expect("Debe procesar el CSV correctamente");

    // 1. Validar cabeceras en snake_case sin tildes
    assert_eq!(
        response.cleaned_headers,
        vec![
            "codigo_empleado",
            "nombre_completo",
            "area_de_trabajo",
            "salario_mensual"
        ]
    );

    // 2. Validar estadísticas
    assert_eq!(response.stats.empty_rows_removed, 1);
    assert_eq!(response.stats.duplicate_rows_removed, 1);
    assert_eq!(response.stats.null_cells_handled, 1);
    assert_eq!(response.stats.cleaned_row_count, 6);

    // 3. Validar exactamente 5 primeras filas en cada vista previa
    assert_eq!(response.original_preview_rows.len(), 5);
    assert_eq!(response.cleaned_preview_rows.len(), 5);

    // 4. Lado izquierdo: primera fila es la original desordenada (500)
    assert_eq!(response.original_preview_rows[0][0].trim(), "500");

    // 5. Lado derecho: las 5 primeras filas están ordenadas por codigo_empleado (100, 200, 300, 400, 500)
    let right_codes: Vec<&str> = response
        .cleaned_preview_rows
        .iter()
        .map(|r| r[0].as_str())
        .collect();
    assert_eq!(right_codes, vec!["100", "200", "300", "400", "500"]);
}

