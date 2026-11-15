//! `GET /anuales/borrar`

use crate::error::ApiError;
use crate::models::entities::anual;
use crate::routes::comun::*;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

#[get("/borrar")]
pub(crate) async fn anual_borrar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<ConId>,
) -> Result<HttpResponse, ApiError> {
	borrar_en_navegador::<anual::Entity>(db.get_ref(), query.id, "inscripcion anual").await
}
