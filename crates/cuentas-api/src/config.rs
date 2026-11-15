//! Configuración que llega por variables de entorno (o por `.env` en desarrollo).

use std::time::Duration;

/// Lo que el servidor necesita para arrancar.
#[derive(Clone, Debug)]
pub struct Config {
	pub database_url: String,
	pub port: u16,
	pub pey: PeySettings,
	/// DynamoDB table for Pey charges and webhooks; without it they are kept in memory.
	pub events_table: Option<String>,
	/// Custom DynamoDB endpoint, e.g. DynamoDB Local.
	pub dynamodb_endpoint: Option<String>,
	/// Month whose fee decides "ya pagó", as `yyyy-mm`.
	pub current_period: String,
	/// Places a workshop offers; Amanda's schema has no column for it yet.
	pub workshop_capacity: u32,
	/// What to seed at start-up: `SEED=1`, `ADMIN_CLAVE`, `DEMO_USUARIO` and `DEMO_CLAVE`.
	pub siembra: crate::services::Siembra,
}

/// How payments reach Pey.
#[derive(Clone, Debug)]
pub struct PeySettings {
	pub live: bool,
	pub secret_id: Option<String>,
	pub base_url: String,
	/// Sends this amount to Pey instead of the real one, for MXN 1 tests.
	pub amount_override: Option<f64>,
	/// Webhook secret for simulate mode and tests; live mode reads it from Secrets Manager.
	pub webhook_secret: Option<String>,
	pub settle_after: Duration,
}

fn var(name: &str) -> Option<String> {
	std::env::var(name)
		.ok()
		.map(|valor| valor.trim().to_string())
		.filter(|valor| !valor.is_empty())
}

impl Config {
	/// Lee `.env` si existe y luego las variables de entorno.
	pub fn from_env() -> Self {
		cargar_env();
		Self {
			database_url: var("DATABASE_URL")
				.unwrap_or_else(|| "postgres://cuentas:cuentas@localhost:5432/cuentas".to_string()),
			port: var("PORT").and_then(|valor| valor.parse().ok()).unwrap_or(8080),
			pey: PeySettings {
				live: var("PEY_MODE").is_some_and(|modo| modo.eq_ignore_ascii_case("live")),
				secret_id: var("PEY_SECRET_ID"),
				base_url: var("PEY_BASE_URL").unwrap_or_else(|| "https://api.pey.mx".to_string()),
				amount_override: var("PEY_AMOUNT_OVERRIDE").and_then(|valor| valor.parse().ok()),
				webhook_secret: var("PEY_WEBHOOK_SECRET"),
				settle_after: Duration::from_secs(4),
			},
			events_table: var("PEY_EVENTS_TABLE"),
			dynamodb_endpoint: var("DYNAMODB_ENDPOINT"),
			current_period: var("CURRENT_PERIOD").unwrap_or_else(|| "2026-12".to_string()),
			workshop_capacity: var("WORKSHOP_CAPACITY")
				.and_then(|valor| valor.parse().ok())
				.unwrap_or(12),
			siembra: crate::services::Siembra {
				datos: var("SEED").is_some_and(|valor| valor == "1" || valor.eq_ignore_ascii_case("true")),
				admin_clave: var("ADMIN_CLAVE"),
				demo: var("DEMO_USUARIO").zip(var("DEMO_CLAVE")),
			},
		}
	}
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

/// Lo que comparten las rutas de la app.
pub struct AppState {
	pub payments: crate::services::cobros::PaymentService,
	pub capacity: u32,
	/// Verifies Pey webhooks; without it the webhook answers 503.
	pub webhook_secret: Option<String>,
}
