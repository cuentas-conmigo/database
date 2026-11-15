//! `GET /talleristas/agregar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{NuevoTallerista, guardar_tallerista};

#[get("/agregar")]
pub(crate) async fn tallerista_agregar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<NuevoTallerista>,
) -> Result<HttpResponse, ApiError> {
	guardar_tallerista(db, web::Json(query.into_inner())).await
}
