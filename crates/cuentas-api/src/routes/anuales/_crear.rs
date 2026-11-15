//! `POST /anuales`

use crate::error::ApiError;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::{NuevaAnual, guardar_anual};

#[post("")]
pub(crate) async fn crear_anual(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevaAnual>,
) -> Result<HttpResponse, ApiError> {
	guardar_anual(db, body).await
}
