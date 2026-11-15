//! `GET /anuales/agregar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{NuevaAnual, guardar_anual};

#[get("/agregar")]
pub(crate) async fn anual_agregar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<NuevaAnual>,
) -> Result<HttpResponse, ApiError> {
	guardar_anual(db, web::Json(query.into_inner())).await
}
