//! `GET /inscripciones/borrar`

use crate::error::ApiError;
use crate::models::entities::inscripcion;
use crate::routes::comun::*;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

#[get("/borrar")]
pub(crate) async fn inscripcion_borrar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<ConId>,
) -> Result<HttpResponse, ApiError> {
	borrar_en_navegador::<inscripcion::Entity>(db.get_ref(), query.id, "inscripcion").await
}
