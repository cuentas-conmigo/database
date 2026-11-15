//! `POST /talleres`

use crate::error::ApiError;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::{NuevoTaller, guardar_taller};

#[post("")]
pub(crate) async fn crear_taller(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevoTaller>,
) -> Result<HttpResponse, ApiError> {
	guardar_taller(db, body).await
}
