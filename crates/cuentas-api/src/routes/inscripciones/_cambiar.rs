//! `PUT /inscripciones/{id}`

use crate::error::ApiError;
use actix_web::{HttpResponse, put, web};
use sea_orm::DatabaseConnection;

use super::{CambioInscripcion, guardar_cambio_inscripcion};

#[put("/{id}")]
pub(crate) async fn cambiar_inscripcion(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
	body: web::Json<CambioInscripcion>,
) -> Result<HttpResponse, ApiError> {
	guardar_cambio_inscripcion(db, id, body).await
}
