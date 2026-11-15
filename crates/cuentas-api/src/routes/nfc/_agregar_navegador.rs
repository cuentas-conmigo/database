//! `GET /nfc/agregar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{NuevoNfc, guardar_nfc};

#[get("/agregar")]
pub(crate) async fn nfc_agregar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<NuevoNfc>,
) -> Result<HttpResponse, ApiError> {
	guardar_nfc(db, web::Json(query.into_inner())).await
}
