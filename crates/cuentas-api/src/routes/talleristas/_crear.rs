//! `POST /talleristas`

use crate::error::ApiError;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::{NuevoTallerista, guardar_tallerista};

#[post("")]
pub(crate) async fn crear_tallerista(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevoTallerista>,
) -> Result<HttpResponse, ApiError> {
	guardar_tallerista(db, body).await
}
