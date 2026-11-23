//! `DELETE /v1/cuenta`

use actix_web::{HttpResponse, delete, web};
use sea_orm::DatabaseConnection;

use crate::error::{AppResult, ErrorBody};
use crate::services::Sesion;

/// Deletes the signed-in instructor's account, their workshops, students' records there and payments.
/// Students who also belong to another instructor's workshop stay for that workshop.
#[utoipa::path(
	tag = "cuenta",
	operation_id = "cuenta_borrar",
	responses(
		(status = 204, description = "The account and its data are gone"),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[delete("")]
pub(super) async fn borrar_cuenta(
	conn: web::Data<DatabaseConnection>,
	sesion: web::ReqData<Sesion>,
) -> AppResult<HttpResponse> {
	crate::services::borrar_cuenta(&conn, sesion.id).await?;
	Ok(HttpResponse::NoContent().finish())
}
