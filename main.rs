mod auth;
mod db;
mod entity;
mod handlers;
mod pagina;

use actix_web::{App, HttpServer, web};

// Durante el desarrollo, Actix recomienda recargar con:
// watchexec -e rs -r cargo run
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cargar_env();
    let db = db::conectar("sqlite:cuentas.db?mode=rwc")
        .await
        .expect("no se pudo abrir cuentas.db");

    let direccion = "0.0.0.0:8080";
    println!("Servidor en http://127.0.0.1:8080");
    println!("También acepta conexiones de otros equipos en el puerto 8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .wrap(auth::ControlAcceso)
            .configure(handlers::config)
    })
    .bind(direccion)?
    .run()
    .await
}

fn cargar_env() {
    let Ok(texto) = std::fs::read_to_string(".env") else {
        return;
    };
    for linea in texto.lines() {
        let linea = linea.trim();
        if linea.is_empty() || linea.starts_with('#') {
            continue;
        }
        let Some((clave, valor)) = linea.split_once('=') else {
            continue;
        };
        let clave = clave.trim();
        let valor = valor.trim().trim_matches('"');
        if !clave.is_empty() && std::env::var(clave).is_err() {
            // El servidor aún no arranca otras tareas, así que escribir el entorno aquí es seguro.
            unsafe { std::env::set_var(clave, valor) };
        }
    }
}
