use actix_web::{App, HttpServer, web};
use cuentas_api::{app, config::Config, middleware::auth, routes};

// Durante el desarrollo, Actix recomienda recargar con:
// watchexec -e rs -r cargo run
#[actix_web::main]
async fn main() -> std::io::Result<()> {
	tracing_subscriber::fmt()
		.with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into()))
		.init();
	let config = Config::from_env();
	let shared = app::build(&config)
		.await
		.map_err(|error| std::io::Error::other(error.to_string()))?;
	let db = shared.db;
	let api = web::Data::new(shared.api);

	let direccion = format!("0.0.0.0:{}", config.port);
	println!("Servidor en http://127.0.0.1:{}", config.port);
	println!(
		"También acepta conexiones de otros equipos en el puerto {}",
		config.port
	);

	HttpServer::new(move || {
		App::new()
			.app_data(web::Data::new(db.clone()))
			.app_data(api.clone())
			.wrap(auth::ControlAcceso)
			.wrap(app::cors())
			.configure(routes::config)
	})
	.bind(direccion)?
	.run()
	.await
}
