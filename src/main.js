/**
 * Limpiador de CSV — Frontend en JavaScript Puro (Vanilla JS)
 * Desarrolladora: Victoria Sanjurjo
 *
 * Responsabilidades:
 * 1. Gestionar la zona Drag & Drop y el selector de archivo enforcing un único archivo CSV por vez.
 * 2. Comunicarse con el backend Rust + Tauri v2 mediante `window.__TAURI__.core.invoke`.
 * 3. Renderizar las 5 primeras filas desordenadas (panel izquierdo) y las 5 primeras filas
 *    ordenadas y limpias (panel derecho) inyectando variables CSS (`--label-N`) para tablas responsivas.
 * 4. Permitir limpiar en 1 clic, cambiar la columna de ordenamiento o el criterio de celdas nulas,
 *    y exportar mediante diálogo nativo "Guardar como...".
 */

(() => {
  "use strict";

  // Estado en memoria del único archivo activo
  const appState = {
    fileName: null,
    rawBytes: null,
    lastResponse: null,
  };

  // Referencias del DOM
  const dropZone = document.getElementById("dropZone");
  const fileInput = document.getElementById("fileInput");
  const dropPrimaryText = document.getElementById("dropPrimaryText");
  const dropSecondaryText = document.getElementById("dropSecondaryText");

  const sortColumnSelect = document.getElementById("sortColumnSelect");
  const nullModeSelect = document.getElementById("nullModeSelect");
  const btnOneClickClean = document.getElementById("btnOneClickClean");
  const btnSaveCsv = document.getElementById("btnSaveCsv");
  const btnLoadDemo = document.getElementById("btnLoadDemo");

  const statusBanner = document.getElementById("statusBanner");
  const statsBar = document.getElementById("statsBar");

  const statDelimiter = document.getElementById("statDelimiter");
  const statEncoding = document.getElementById("statEncoding");
  const statRows = document.getElementById("statRows");
  const statRemoved = document.getElementById("statRemoved");
  const statCells = document.getElementById("statCells");

  const rawTable = document.getElementById("rawTable");
  const rawThead = document.getElementById("rawThead");
  const rawTbody = document.getElementById("rawTbody");
  const rawPanelMeta = document.getElementById("rawPanelMeta");

  const cleanTable = document.getElementById("cleanTable");
  const cleanThead = document.getElementById("cleanThead");
  const cleanTbody = document.getElementById("cleanTbody");
  const cleanPanelMeta = document.getElementById("cleanPanelMeta");

  /**
   * Obtiene la función `invoke` de Tauri v2 si se ejecuta dentro del contenedor nativo.
   */
  function getTauriInvoke() {
    if (
      window.__TAURI__ &&
      window.__TAURI__.core &&
      typeof window.__TAURI__.core.invoke === "function"
    ) {
      return window.__TAURI__.core.invoke;
    }
    return null;
  }

  /**
   * Muestra un mensaje informativo, de éxito o de error en el banner superior.
   */
  function showStatus(message, type = "info") {
    statusBanner.textContent = message;
    statusBanner.className = `status-banner status-${type}`;
  }

  /**
   * Valida que se haya entregado exactamente 1 archivo y que tenga extensión `.csv`.
   */
  function validateSingleCsvFile(fileList) {
    if (!fileList || fileList.length === 0) {
      return { valid: false, error: "No se seleccionó ningún archivo." };
    }
    if (fileList.length > 1) {
      return {
        valid: false,
        error:
          "Solo se puede subir 1 archivo CSV por vez para no sobrecargar la aplicación.",
      };
    }
    const file = fileList[0];
    if (!file.name.toLowerCase().endsWith(".csv")) {
      return {
        valid: false,
        error: `El archivo "${file.name}" no tiene extensión .csv. Por favor, sube únicamente un archivo CSV.`,
      };
    }
    return { valid: true, file };
  }

  /**
   * Asigna las variables CSS `--label-1`, `--label-2`, etc. a la tabla para cumplir
   * con el diseño apilado accesible en contenedores estrechos (`responsive-table`).
   */
  function applyResponsiveColumnVariables(tableEl, headers) {
    headers.forEach((h, idx) => {
      const safeLabel = String(h || `Col ${idx + 1}`).replace(/"/g, "'");
      tableEl.style.setProperty(`--label-${idx + 1}`, `"${safeLabel}"`);
    });
  }

  /**
   * Renderiza una tabla de vista previa (máximo 5 primeras filas) con cabeceras semánticas.
   */
  function renderPreviewTable({
    tableEl,
    theadEl,
    tbodyEl,
    headers,
    rows,
    isRawPanel,
    sortedColIdx,
  }) {
    applyResponsiveColumnVariables(tableEl, headers);

    // Construir <thead>
    theadEl.innerHTML = "";
    const trHead = document.createElement("tr");

    const thIndex = document.createElement("th");
    thIndex.scope = "col";
    thIndex.textContent = "Fila";
    trHead.appendChild(thIndex);

    headers.forEach((headerText, colIdx) => {
      const th = document.createElement("th");
      th.scope = "col";
      if (!isRawPanel && colIdx === sortedColIdx) {
        th.textContent = `${headerText} ↑`;
        th.title = "Columna utilizada para ordenar ascendentemente";
      } else {
        th.textContent = headerText || `(vacia_${colIdx + 1})`;
      }
      trHead.appendChild(th);
    });
    theadEl.appendChild(trHead);

    // Construir <tbody> (5 primeras filas)
    tbodyEl.innerHTML = "";
    if (!rows || rows.length === 0) {
      const trEmpty = document.createElement("tr");
      trEmpty.className = "empty-state-row";
      const thRow = document.createElement("th");
      thRow.scope = "row";
      thRow.textContent = "—";
      const tdMsg = document.createElement("td");
      tdMsg.colSpan = Math.max(1, headers.length);
      tdMsg.textContent = "No hay filas para mostrar.";
      trEmpty.appendChild(thRow);
      trEmpty.appendChild(tdMsg);
      tbodyEl.appendChild(trEmpty);
      return;
    }

    rows.slice(0, 5).forEach((rowCells, rowIdx) => {
      const tr = document.createElement("tr");

      const thRowNum = document.createElement("th");
      thRowNum.scope = "row";
      thRowNum.textContent = `#${rowIdx + 1}`;
      tr.appendChild(thRowNum);

      headers.forEach((_, colIdx) => {
        const td = document.createElement("td");
        const val = rowCells[colIdx] !== undefined ? String(rowCells[colIdx]) : "";

        if (isRawPanel) {
          // Resaltar visualmente si la celda original tenía espacios extra o estaba vacía/NaN
          const hasLeadingOrTrailingSpace = val !== val.trim() && val.length > 0;
          const isEmptyOrNull =
            val.trim() === "" ||
            ["nan", "null", "n/a", "none", "-"].includes(val.trim().toLowerCase());

          if (isEmptyOrNull) {
            const badge = document.createElement("span");
            badge.className = "cell-dirty-badge";
            badge.textContent = val.trim() === "" ? "(vacío)" : val;
            td.appendChild(badge);
          } else if (hasLeadingOrTrailingSpace) {
            const badge = document.createElement("span");
            badge.className = "cell-dirty-badge";
            badge.title = "Contiene espacios en blanco innecesarios";
            badge.textContent = `␣${val.trim()}␣`;
            td.appendChild(badge);
          } else {
            td.textContent = val;
          }
        } else {
          if (colIdx === sortedColIdx) {
            td.classList.add("cell-sorted");
          }
          td.textContent = val;
        }

        tr.appendChild(td);
      });

      tbodyEl.appendChild(tr);
    });
  }

  /**
   * Actualiza el selector de columnas cuando se carga un nuevo archivo CSV.
   */
  function populateSortSelector(cleanedHeaders, selectedIdx) {
    sortColumnSelect.innerHTML = "";
    cleanedHeaders.forEach((h, idx) => {
      const opt = document.createElement("option");
      opt.value = String(idx);
      opt.textContent = `${idx + 1}. ${h}`;
      if (idx === selectedIdx) {
        opt.selected = true;
      }
      sortColumnSelect.appendChild(opt);
    });
    sortColumnSelect.disabled = false;
  }

  /**
   * Invoca al backend de Rust (Tauri v2) para limpiar y ordenar el CSV activo.
   */
  async function executeClean(sortColIndex = 0, triggeredByOneClickButton = false) {
    if (!appState.rawBytes || !appState.fileName) {
      return;
    }

    const invoke = getTauriInvoke();
    if (!invoke) {
      showStatus(
        "No se detectó el entorno nativo Tauri v2 (window.__TAURI__.core.invoke).",
        "error"
      );
      return;
    }

    try {
      const nullMode = nullModeSelect.value;
      const response = await invoke("clean_csv_payload", {
        fileName: appState.fileName,
        rawBytes: Array.from(appState.rawBytes),
        sortCol: sortColIndex,
        nullMode: nullMode,
      });

      appState.lastResponse = response;

      // Actualizar selector de columna de ordenamiento
      populateSortSelector(response.cleaned_headers, response.sort_column_index);

      // Renderizar Panel Izquierdo (5 primeras filas desordenadas del original)
      renderPreviewTable({
        tableEl: rawTable,
        theadEl: rawThead,
        tbodyEl: rawTbody,
        headers: response.original_headers,
        rows: response.original_preview_rows,
        isRawPanel: true,
        sortedColIdx: response.sort_column_index,
      });
      rawPanelMeta.textContent = `Mostrando ${response.original_preview_rows.length} de ${response.stats.original_row_count} filas originales`;

      // Renderizar Panel Derecho (5 primeras filas ordenadas y limpias)
      renderPreviewTable({
        tableEl: cleanTable,
        theadEl: cleanThead,
        tbodyEl: cleanTbody,
        headers: response.cleaned_headers,
        rows: response.cleaned_preview_rows,
        isRawPanel: false,
        sortedColIdx: response.sort_column_index,
      });
      const sortedHeaderName =
        response.cleaned_headers[response.sort_column_index] || "Col 1";
      cleanPanelMeta.textContent = `Mostrando ${response.cleaned_preview_rows.length} de ${response.stats.cleaned_row_count} filas · Orden: ${sortedHeaderName} (A→Z)`;

      // Actualizar barra de estadísticas
      statsBar.classList.remove("hidden");
      statDelimiter.textContent = `${response.stats.detected_delimiter} → Coma (,)`;
      statEncoding.textContent = response.stats.detected_encoding;
      statRows.textContent = `${response.stats.original_row_count} → ${response.stats.cleaned_row_count}`;
      statRemoved.textContent = `${response.stats.empty_rows_removed} vacías / ${response.stats.duplicate_rows_removed} dup.`;
      statCells.textContent = `${response.stats.trimmed_cells_count} trim / ${response.stats.null_cells_handled} nulos`;

      btnOneClickClean.disabled = false;
      btnSaveCsv.disabled = false;

      if (triggeredByOneClickButton) {
        // Si el usuario pulsó "Limpiar con un solo clic", además de limpiar abrimos el diálogo para guardar inmediatamente
        await promptSaveCleanedCsv();
      } else {
        showStatus(
          `Archivo "${response.file_name}" analizado y limpiado automáticamente. Compará las 5 primeras filas o hacé clic en "Limpiar con un solo clic" para exportar.`,
          "success"
        );
      }
    } catch (err) {
      showStatus(`Error al procesar el CSV: ${err}`, "error");
    }
  }

  /**
   * Abre el diálogo nativo "Guardar como..." de Windows sugiriendo `<nombre_original>_limpio.csv`.
   */
  async function promptSaveCleanedCsv() {
    if (!appState.lastResponse) {
      return;
    }
    const invoke = getTauriInvoke();
    if (!invoke) {
      return;
    }

    try {
      const saveRes = await invoke("save_cleaned_csv", {
        originalFileName: appState.lastResponse.file_name,
        cleanedCsvContent: appState.lastResponse.cleaned_csv_content,
      });

      if (saveRes.saved && saveRes.file_path) {
        showStatus(
          `¡CSV limpio y ordenado guardado con éxito en: ${saveRes.file_path}!`,
          "success"
        );
      } else {
        showStatus(
          "Limpieza aplicada en vista previa (el diálogo de guardado fue cancelado).",
          "info"
        );
      }
    } catch (err) {
      showStatus(`No se pudo guardar el archivo CSV: ${err}`, "error");
    }
  }

  /**
   * Carga un objeto `File` desde Drag & Drop o desde el input selector.
   */
  async function loadCsvFileObject(file) {
    const buffer = await file.arrayBuffer();
    appState.fileName = file.name;
    appState.rawBytes = new Uint8Array(buffer);

    dropPrimaryText.innerHTML = `Archivo cargado: <code>${file.name}</code> (${(
      file.size / 1024
    ).toFixed(1)} KB)`;
    dropSecondaryText.textContent =
      "Podés arrastrar otro archivo .csv en cualquier momento para reemplazarlo (1 archivo por vez).";

    await executeClean(0, false);
  }

  // Eventos Drag & Drop y Teclado
  dropZone.addEventListener("click", () => {
    fileInput.value = "";
    fileInput.click();
  });

  dropZone.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      fileInput.value = "";
      fileInput.click();
    }
  });

  fileInput.addEventListener("change", (e) => {
    const validation = validateSingleCsvFile(e.target.files);
    if (!validation.valid) {
      showStatus(validation.error, "error");
      return;
    }
    loadCsvFileObject(validation.file);
  });

  ["dragenter", "dragover"].forEach((eventName) => {
    dropZone.addEventListener(eventName, (e) => {
      e.preventDefault();
      e.stopPropagation();
      dropZone.classList.add("drag-over");
    });
  });

  ["dragleave", "drop"].forEach((eventName) => {
    dropZone.addEventListener(eventName, (e) => {
      e.preventDefault();
      e.stopPropagation();
      dropZone.classList.remove("drag-over");
    });
  });

  dropZone.addEventListener("drop", (e) => {
    const dt = e.dataTransfer;
    const files = dt ? dt.files : null;
    const validation = validateSingleCsvFile(files);
    if (!validation.valid) {
      showStatus(validation.error, "error");
      return;
    }
    loadCsvFileObject(validation.file);
  });

  // Prevenir que soltar un archivo fuera del área recargue el WebView
  window.addEventListener("dragover", (e) => e.preventDefault());
  window.addEventListener("drop", (e) => e.preventDefault());

  // Cambios de configuración reactivos
  sortColumnSelect.addEventListener("change", () => {
    const colIdx = parseInt(sortColumnSelect.value, 10) || 0;
    executeClean(colIdx, false);
  });

  nullModeSelect.addEventListener("change", () => {
    const colIdx = parseInt(sortColumnSelect.value, 10) || 0;
    executeClean(colIdx, false);
  });

  // Botón principal "Limpiar con un solo clic"
  btnOneClickClean.addEventListener("click", () => {
    const colIdx = parseInt(sortColumnSelect.value, 10) || 0;
    executeClean(colIdx, true);
  });

  // Botón secundario "Guardar CSV limpio..."
  btnSaveCsv.addEventListener("click", () => {
    promptSaveCleanedCsv();
  });

  // Botón "Cargar CSV de prueba" para demostración inmediata en un clic
  btnLoadDemo.addEventListener("click", async () => {
    const sampleCsv = [
      "  ID Venta ; Nombre del Cliente ; Categoría Producto ; Monto Total ($) ; Fecha Operación ",
      "104 ;   Sofía  Martínez  ; Electrónica ; 1850.75 ; 2026-10-14",
      " ; ; ; ; ",
      "101 ; Martín   Gómez ; Hogar y Deco ; 920.00 ; 2026-10-10",
      "101 ; Martín Gómez ; Hogar y Deco ; 920.00 ; 2026-10-10",
      "105 ; Valentina López ; Indumentaria ; NaN ; 2026-10-18",
      "102 ;   Lucas Fernández ; Electrónica ; 3400.50 ; 2026-10-11",
      "103 ; Camila   Ruiz ; Librería ; 410.25 ; 2026-10-12",
      "106 ; Mateo Díaz ; Deportes ; 1290.00 ; 2026-10-19",
    ].join("\n");

    const encoder = new TextEncoder();
    appState.fileName = "ventas_desordenadas_ejemplo.csv";
    appState.rawBytes = encoder.encode(sampleCsv);

    dropPrimaryText.innerHTML = `Archivo de prueba cargado: <code>${appState.fileName}</code>`;
    dropSecondaryText.textContent =
      "Contiene separadores ';', espacios extra, tildes en cabeceras, fila vacía, fila duplicada, valor NaN y filas desordenadas.";

    await executeClean(0, false);
  });
})();

