//! `POST /nfc`

use crate::error::ApiError;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::{NuevoNfc, guardar_nfc};

#[post("")]
pub(crate) async fn crear_nfc(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevoNfc>,
) -> Result<HttpResponse, ApiError> {
	guardar_nfc(db, body).await
}
