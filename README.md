# Limpiador de CSV

**Limpiador de CSV** es una utilidad nativa de escritorio para Windows desarrollada con **Rust + Tauri v2** (backend) y **HTML5, CSS3 y JavaScript puro** (frontend). Permite depurar, normalizar y ordenar archivos CSV **con un solo clic**, mostrando una **visualización comparativa doble** con las **5 primeras filas desordenadas** del archivo original en el panel izquierdo y las **5 primeras filas ordenadas y limpias** en el panel derecho.

- **Nombre de la aplicación:** Limpiador de CSV
- **Desarrolladora:** Victoria Sanjurjo
- **Publisher ID (MSIX):** `CN=CF220D35-0C1C-408C-BCB2-E85AA65BC95B`
- **Licencia:** MIT

---

## 1. ¿Para qué sirve y qué problema resuelve?

Los archivos CSV provenientes de distintas fuentes (exportaciones de ERP, hojas de cálculo en español con separador `;`, codificaciones ANSI/Windows-1252, formularios web) suelen presentar inconsistencias que dificultan su análisis o importación:
- Espacios en blanco adicionales antes, después o en medio de los textos.
- Filas completamente vacías intercaladas o registros duplicados exactos.
- Separadores heterogéneos (`;`, `\t`, `|`) y problemas con tildes/ñ por codificación `Windows-1252`.
- Valores nulos o faltantes (`NaN`, `null`, `N/A`, celdas vacías).
- Cabeceras con símbolos, mayúsculas mezcladas y tildes que complican consultas SQL o scripts.
- Registros desordenados.

**Limpiador de CSV** resuelve estos problemas aplicando **las 5 reglas de limpieza + ordenamiento en un solo clic**, procesando únicamente **1 archivo por vez** para garantizar ligereza e inmediatez.

---

## 2. Características Principales

1. **Carga mediante Drag & Drop (1 archivo por vez):**
   - Zona interactiva para arrastrar y soltar archivos `.csv` (o seleccionarlos con clic/teclado).
   - Validación estricta para rechazar múltiples archivos simultáneos o extensiones distintas de `.csv`.
2. **Visualización Doble de las 5 Primeras Filas:**
   - **Panel Izquierdo (Antes de limpiar):** Muestra las 5 primeras filas tal como vienen en el archivo original (desordenadas), resaltando con etiquetas visuales las celdas con espacios extra (`␣texto␣`) o valores vacíos/`NaN`.
   - **Panel Derecho (Después de limpiar):** Muestra las 5 primeras filas tras aplicar todas las reglas de limpieza y el ordenamiento ascendente.
3. **Motor de Limpieza de 5 Reglas en Rust:**
   - **Regla 1 (Trim y colapso de espacios):** Elimina espacios iniciales/finales y reduce espacios múltiples internos a uno solo en cabeceras y celdas.
   - **Regla 2 (Sin filas vacías ni duplicadas):** Descarta filas completamente en blanco y elimina filas duplicadas exactas.
   - **Regla 3 (Normalización de separadores y UTF-8):** Detecta automáticamente `,`, `;`, `\t` o `|` y la codificación (`UTF-8` o `Windows-1252`), generando siempre salida estándar separada por `,` en `UTF-8`.
   - **Regla 4 (Tratamiento de celdas vacías / NaN):** Permite elegir entre rellenar celdas vacías/`NaN`/`null` con `"N/D"` (por defecto) o descartar filas incompletas.
   - **Regla 5 (Cabeceras en `snake_case` sin tildes):** Convierte los nombres de las columnas a `snake_case` limpio (`Año de Facturación ($)` → `ano_de_facturacion`).
4. **Ordenamiento Inteligente y Selector de Columna:**
   - Ordena automáticamente por la **primera columna** de forma ascendente (comparando números como valores reales y texto alfabéticamente) y permite cambiar la columna de ordenamiento desde la interfaz.
5. **Exportación Nativa:**
   - Abre el diálogo nativo de Windows **"Guardar como..."** sugiriendo automáticamente `<nombre_original>_limpio.csv`.

---

## 3. Arquitectura y Componentes

| Capa | Tecnología | Archivos Clave | Responsabilidad |
| :--- | :--- | :--- | :--- |
| **Frontend UI** | HTML5 + CSS3 + JS Puro | `src/index.html`, `src/styles.css`, `src/main.js` | Interfaz editorial clara, Drag & Drop de 1 archivo, tablas responsivas con `@container` y cabeceras `sticky`, comunicación IPC con Tauri. |
| **Backend IPC** | Rust + Tauri v2 | `src-tauri/src/lib.rs`, `src-tauri/src/main.rs` | Exposición de comandos `clean_csv_payload` y `save_cleaned_csv`, diálogo nativo `rfd::FileDialog`. |
| **Motor CSV** | Rust (`csv`, `encoding_rs`, `unicode-normalization`) | `src-tauri/src/cleaner.rs` | Decodificación UTF-8/ANSI, autodetección de delimitador, aplicación de las 5 reglas de limpieza, ordenamiento y extracción de las 5 primeras filas. |
| **Empaquetado MSIX** | Windows SDK (`makeappx`, `signtool`) | `config/AppxManifest.xml`, `config/package_msix.ps1` | Generación y firma digital del instalador `outputs/LimpiadorDeCSV_1.0.0.0_x64.msix` con el Publisher ID oficial. |

---

## 4. Árbol de Directorios del Proyecto

```text
Limpiador de CSV/
├── assets/                                  # Íconos originales de la aplicación (.svg, .png, .ico)
│   ├── icon.ico
│   ├── icon.png
│   └── icon.svg
├── config/                                  # Configuración de la app y manifiesto/script de empaquetado MSIX
│   ├── app_config.json
│   ├── AppxManifest.xml
│   └── package_msix.ps1
├── docs/                                    # Documentación técnica y decisiones de arquitectura
│   └── ARQUITECTURA.md
├── outputs/                                 # Paquete MSIX compilado y certificado público de firma
│   ├── LimpiadorDeCSV_1.0.0.0_x64.msix
│   └── VictoriaSanjurjo_LimpiadorDeCSV.cer
├── src/                                     # Frontend en HTML, CSS y JavaScript puro (Vanilla)
│   ├── index.html
│   ├── main.js
│   └── styles.css
├── src-tauri/                               # Backend en Rust + Tauri v2
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/                               # Set completo de íconos Tauri y MSIX (StoreLogo, Square150x150, etc.)
│   ├── src/
│   │   ├── cleaner.rs                       # Motor de limpieza y pruebas unitarias
│   │   ├── lib.rs                           # Comandos Tauri v2
│   │   └── main.rs                          # Punto de entrada binario
│   ├── tests/
│   │   └── integration_cleaner_test.rs      # Pruebas de integración del motor CSV
│   ├── build.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── tests/                                   # Datos CSV de prueba con errores reales para validación
│   └── muestra_sucia.csv
├── .gitignore
├── LICENSE.txt                              # Licencia MIT (Victoria Sanjurjo)
└── README.md                                # Este documento
```

---

## 5. Instrucciones Paso a Paso para Ejecutar, Probar y Empaquetar

### Requisitos Previos
- **Sistema Operativo:** Windows 10 (1809+) o Windows 11 (x64) con WebView2 Runtime (incluido por defecto en Windows).
- **Toolchain Rust:** `rustc` y `cargo` instalados.
- **Tauri CLI v2:** `cargo tauri` disponible.
- **Windows 10/11 SDK:** Para empaquetar y firmar el `.msix` (`makeappx.exe` y `signtool.exe`).

### Paso 1: Ejecutar las pruebas automatizadas (Unitarias e Integración)
```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

### Paso 2: Ejecutar en modo desarrollo
```powershell
cargo tauri dev
```

### Paso 3: Compilar el ejecutable optimizado de producción (`Release`)
```powershell
cargo build --release --manifest-path src-tauri/Cargo.toml
```
El ejecutable nativo quedará disponible en `src-tauri/target/release/limpiador-de-csv.exe`.

### Paso 4: Generar y firmar el paquete instalador MSIX
Ejecuta el script automatizado de empaquetado MSIX:
```powershell
powershell -ExecutionPolicy Bypass -File config/package_msix.ps1
```
Esto generará en la carpeta `outputs/`:
- `outputs/LimpiadorDeCSV_1.0.0.0_x64.msix` (Empaquetado con `Publisher="CN=CF220D35-0C1C-408C-BCB2-E85AA65BC95B"` y `PublisherDisplayName="Victoria Sanjurjo"`).
- `outputs/VictoriaSanjurjo_LimpiadorDeCSV.cer` (Certificado autofirmado empleado para la firma SHA256).

