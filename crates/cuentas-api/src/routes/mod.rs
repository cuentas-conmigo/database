//! Rutas HTTP. Cada carpeta es un recurso: su `mod.rs` registra el recurso y cada archivo `_accion.rs`
//! tiene una sola ruta.

mod acceso;
mod alumnos;
mod anuales;
pub(crate) mod comun;
mod cuenta;
mod cuenta_app;
mod inscripciones;
mod nfc;
mod pagos;
mod reportes;
mod salud;
mod sesion;
mod talleres;
mod talleristas;
mod webhooks;

#[cfg(test)]
mod tests;

use actix_web::web;

/// Registra todo: las páginas HTML de Amanda en la raíz y todas las rutas JSON bajo `/v1`.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.configure(acceso::config).configure(cuenta::config).service(
		web::scope("/v1")
			.configure(salud::config)
			.configure(sesion::config)
			.configure(cuenta_app::config)
			.configure(webhooks::config)
			.configure(reportes::config)
			.configure(alumnos::config)
			.configure(talleristas::config)
			.configure(talleres::config)
			.configure(nfc::config)
			.configure(anuales::config)
			.configure(inscripciones::config)
			.configure(pagos::config),
	);
}

/// Fragmentos de OpenAPI de las rutas de la app, que junta [`crate::docs::ApiDoc`].
pub mod docs {
	pub use super::alumnos::ApiDoc as Alumnos;
	pub use super::cuenta_app::ApiDoc as Cuenta;
	pub use super::pagos::ApiDoc as Pagos;
	pub use super::salud::ApiDoc as Salud;
	pub use super::sesion::ApiDoc as Sesion;
	pub use super::talleres::ApiDoc as Talleres;
	pub use super::webhooks::ApiDoc as Webhooks;
}
