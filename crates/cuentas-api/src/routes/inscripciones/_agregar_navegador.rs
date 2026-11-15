//! `GET /inscripciones/agregar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{NuevaInscripcion, guardar_inscripcion};

#[get("/agregar")]
pub(crate) async fn inscripcion_agregar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<NuevaInscripcion>,
) -> Result<HttpResponse, ApiError> {
	guardar_inscripcion(db, web::Json(query.into_inner())).await
}
