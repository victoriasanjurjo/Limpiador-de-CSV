# Arquitectura y Decisiones de Diseño — Limpiador de CSV

## 1. Propósito del Sistema

**Limpiador de CSV** es una utilidad nativa de escritorio para Windows diseñada por **Victoria Sanjurjo** cuyo objetivo es depurar, estandarizar y ordenar archivos CSV en un solo clic. Permite comparar de forma visual e inmediata las **5 primeras filas desordenadas** del archivo original (panel izquierdo) frente a las **5 primeras filas ordenadas y limpias** (panel derecho).

---

## 2. Decisiones Técnicas y Justificación ("El Por Qué")

### 2.1. Backend en Rust + Tauri v2 (`src-tauri/`)
- **Por qué Tauri v2 sobre Electron:** Tauri v2 utiliza el componente nativo WebView2 del sistema operativo Windows y un backend compilado en Rust, reduciendo drásticamente el consumo de memoria RAM y el tamaño del binario ejecutable (`.exe` y paquete `.msix`).
- **Por qué `csv = "1.3"`:** Es el parser CSV de referencia en el ecosistema Rust, capaz de manejar campos entrecomillados, saltos de línea embebidos y registros de longitud flexible (`flexible(true)`).
- **Por qué `encoding_rs = "0.8"`:** Muchos archivos CSV generados por Excel en sistemas Windows en español se guardan en codificación `Windows-1252` (ANSI / Latin-1) en lugar de `UTF-8`. `encoding_rs` permite detectar si los bytes son UTF-8 válidos (con o sin BOM) y hacer fallback transparente a `Windows-1252`, exportando siempre en `UTF-8` estándar.
- **Por qué `unicode-normalization = "0.1"`:** Para estandarizar los nombres de columnas a `snake_case` sin tildes ni diéresis (`Año de Facturación` → `ano_de_facturacion`), se aplica descomposición canónica Unicode (`NFD`) y filtrado de marcas combinatorias (`is_combining_mark`), garantizando compatibilidad con bases de datos SQL y pipelines de datos.
- **Por qué `rfd = "0.15"` (Rusty File Dialog):** Provee invocación directa a las APIs nativas de diálogo `IFileSaveDialog` de Windows (`Guardar como...`) sin requerir permisos complejos en el frontend, sugiriendo automáticamente `<nombre_original>_limpio.csv`.

### 2.2. Frontend en HTML5, CSS3 y JavaScript Puro (`src/`)
- **Por qué Vanilla HTML/CSS/JS sin frameworks pesados:** Cumple estrictamente con el requerimiento de diseño ligero y sin sobrecarga de transpilación. Al habilitar `withGlobalTauri: true` en `tauri.conf.json`, el frontend se comunica de forma directa con Rust mediante `window.__TAURI__.core.invoke`.
- **Por qué `dragDropEnabled: false` en la ventana de Tauri:** En Tauri v2, el interceptor nativo de arrastrar y soltar de la ventana captura los eventos de archivo antes que el DOM HTML5. Al deshabilitar el interceptor de ventana (`dragDropEnabled: false`), el elemento `#dropZone` del DOM recibe directamente el objeto `DataTransfer.files`, permitiendo validar inmediatamente que solo se arrastre **1 archivo `.csv` por vez** y leer su `ArrayBuffer` de manera instantánea.
- **Accesibilidad y Tablas Responsivas (`responsive-table`):** Se implementaron contenedores desplazables con `tabindex="0"`, `role="region"`, `aria-labelledby`, cabeceras fijas (`position: sticky` tanto en `thead` como en `tbody th`) y `@container (width < 480px)` con inyección de etiquetas CSS (`--label-N`).

### 2.3. Reglas del Motor de Limpieza (`src-tauri/src/cleaner.rs`)
El pipeline ejecuta secuencialmente:
1. **Decodificación y detección de separador:** Evalúa `,`, `;`, `\t` y `|` en las primeras líneas no vacías y normaliza la salida a `,` en UTF-8.
2. **Estandarización de cabeceras:** Recorta espacios y transforma cada columna a `snake_case` sin tildes, añadiendo sufijos incrementales (`_2`, `_3`) si existieran columnas con nombres repetidos.
3. **Limpieza de celdas (Trim y colapso):** Recorta espacios iniciales/finales y colapsa múltiples espacios internos consecutivos en uno solo.
4. **Filtrado de filas vacías y tratamiento de nulos (`NaN`, `null`, `N/A`, vacíos):**
   - Elimina cualquier fila donde todas las celdas estén vacías.
   - Para celdas individuales vacías o con marcadores nulos (`NaN`, `null`, `N/A`), permite elegir entre rellenar con `"N/D"` (por defecto) o descartar la fila incompleta.
5. **Deduplicación exacta:** Mantiene un `HashSet<Vec<String>>` para descartar filas duplicadas exactas preservando únicamente la primera aparición.
6. **Ordenamiento inteligente:** Ordena ascendentemente por la columna elegida (por defecto la primera columna, índice `0`), comparando numéricamente (`f64`) cuando ambos valores son números válidos y alfabéticamente (case-insensitive) en caso contrario.

---

## 3. Empaquetado MSIX y Firma Digital

Para distribuir la aplicación en Windows bajo los parámetros solicitados:
- **Nombre de la aplicación:** `Limpiador de CSV`
- **Desarrolladora:** `Victoria Sanjurjo`
- **Publisher ID:** `CN=CF220D35-0C1C-408C-BCB2-E85AA65BC95B`

Se incluye el script automatizado `config/package_msix.ps1` que:
1. Prepara el directorio de empaquetado con el ejecutable compilado en modo `release` (`limpiador-de-csv.exe`), los activos visuales (`Assets\StoreLogo.png`, `Square44x44Logo.png`, `Square150x150Logo.png`, `Wide310x150Logo.png`) y el manifiesto `AppxManifest.xml`.
2. Ejecuta `makeappx.exe pack` del Windows 10/11 SDK para construir el paquete `outputs/LimpiadorDeCSV_1.0.0.0_x64.msix`.
3. Genera un certificado autofirmado con `Subject = "CN=CF220D35-0C1C-408C-BCB2-E85AA65BC95B"` y firma digitalmente el paquete `.msix` mediante `signtool.exe sign /fd SHA256`.

