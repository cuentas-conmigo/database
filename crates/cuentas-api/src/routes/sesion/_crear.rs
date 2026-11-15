//! `POST /v1/sesion`

use actix_web::{HttpRequest, HttpResponse, post, web};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::error::{AppError, AppResult, ErrorBody};
use crate::models::dto::{SessionBody, SessionDto};
use crate::models::entities::taller;
use crate::services::acceso;

/// Checks the login (body or Basic header) and answers who it is and their first workshop.
#[utoipa::path(
	tag = "sesion",
	operation_id = "sesion_crear",
	request_body = SessionBody,
	responses(
		(status = 200, description = "The tallerista and their first workshop", body = SessionDto),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
	),
)]
#[post("")]
pub(super) async fn crear_sesion(
	req: HttpRequest,
	conn: web::Data<DatabaseConnection>,
	body: Option<web::Json<SessionBody>>,
) -> AppResult<HttpResponse> {
	let from_body = body.and_then(|body| {
		let body = body.into_inner();
		body.usuario.zip(body.clave)
	});
	let sesion = match from_body {
		Some((usuario, clave)) => acceso::tallerista(&conn, &usuario, &clave).await?,
		None => {
			let header = req
				.headers()
				.get(actix_web::http::header::AUTHORIZATION)
				.and_then(|value| value.to_str().ok());
			acceso::tallerista_basic(&conn, header).await?
		}
	}
	.ok_or_else(AppError::unauthorized)?;
	let first = taller::Entity::find()
		.filter(taller::Column::TalleristaId.eq(sesion.id))
		.filter(taller::Column::Activo.eq(1))
		.order_by_asc(taller::Column::Id)
		.one(conn.get_ref())
		.await?;
	Ok(HttpResponse::Ok().json(SessionDto {
		name: sesion.nombre,
		role: "instructor",
		workshop_id: first.map(|row| row.id.to_string()),
	}))
}
