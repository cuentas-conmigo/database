//! `POST /inscripciones`

use crate::error::ApiError;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::{NuevaInscripcion, guardar_inscripcion};

#[post("")]
pub(crate) async fn crear_inscripcion(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevaInscripcion>,
) -> Result<HttpResponse, ApiError> {
	guardar_inscripcion(db, body).await
}
