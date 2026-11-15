mod db;
mod entity;
mod handlers;

use actix_web::{App, HttpServer, web};

// Durante el desarrollo, Actix recomienda recargar con:
// watchexec -e rs -r cargo run
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = db::conectar("sqlite:cuentas.db?mode=rwc")
        .await
        .expect("no se pudo abrir cuentas.db");

    let direccion = "0.0.0.0:8080";
    println!("Servidor en http://127.0.0.1:8080");
    println!("También acepta conexiones de otros equipos en el puerto 8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .configure(handlers::config)
    })
    .bind(direccion)?
    .run()
    .await
}
