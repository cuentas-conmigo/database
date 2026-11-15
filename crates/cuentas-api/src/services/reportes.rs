//! Reportes de ingresos.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};

#[derive(Debug, serde::Serialize)]
pub struct Ingreso {
	pub nombre: String,
	pub oficio: String,
	pub total_cobrado_mxn: i32,
}

pub async fn ingresos(db: &DatabaseConnection) -> Result<Vec<Ingreso>, DbErr> {
	let filas = db
		.query_all_raw(Statement::from_string(
			DbBackend::Postgres,
			"SELECT nombre, oficio, total_cobrado_mxn
             FROM ingresos_tallerista
             ORDER BY tallerista_id"
				.to_owned(),
		))
		.await?;
	filas
		.into_iter()
		.map(|fila| {
			Ok(Ingreso {
				nombre: fila.try_get("", "nombre")?,
				oficio: fila.try_get("", "oficio")?,
				total_cobrado_mxn: fila.try_get("", "total_cobrado_mxn")?,
			})
		})
		.collect()
}
