//! `GET /alumnos/cambiar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{CambioAlumno, CambioAlumnoDireccion, guardar_cambio_alumno};

#[get("/cambiar")]
pub(crate) async fn alumno_cambiar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<CambioAlumnoDireccion>,
) -> Result<HttpResponse, ApiError> {
	let datos = query.into_inner();
	guardar_cambio_alumno(
		db,
		web::Path::from(datos.id),
		web::Json(CambioAlumno {
			nombre: datos.nombre,
			cartera_ref: datos.cartera_ref,
		}),
	)
	.await
}
