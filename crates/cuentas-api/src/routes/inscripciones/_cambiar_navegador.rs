//! `GET /inscripciones/cambiar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{CambioInscripcion, CambioInscripcionDireccion, guardar_cambio_inscripcion};

#[get("/cambiar")]
pub(crate) async fn inscripcion_cambiar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<CambioInscripcionDireccion>,
) -> Result<HttpResponse, ApiError> {
	let datos = query.into_inner();
	guardar_cambio_inscripcion(
		db,
		web::Path::from(datos.id),
		web::Json(CambioInscripcion { activa: datos.activa }),
	)
	.await
}
