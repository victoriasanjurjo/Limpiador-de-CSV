// Evita abrir una ventana de consola adicional en Windows en modo release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    limpiador_de_csv_lib::run()
}

