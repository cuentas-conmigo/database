//! `GET /v1/talleres/{id}/cobros`

use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use crate::config::AppState;
use crate::error::{AppResult, ErrorBody};
use crate::models::dto::ChargeOptionDto;
use crate::routes::comun::id;
use crate::services::Sesion;

/// What a student can be charged now; amounts follow `PEY_AMOUNT_OVERRIDE` when it is set.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_cobros",
	params(("id" = String, Path, description = "Workshop id")),
	responses(
		(status = 200, description = "Charge options", body = [ChargeOptionDto]),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "Not a workshop of this tallerista", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[get("/{id}/cobros")]
pub(super) async fn cobros_del_taller(
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	sesion: web::ReqData<Sesion>,
	path: web::Path<String>,
) -> AppResult<HttpResponse> {
	let row = state
		.payments
		.workshop(&conn, sesion.id, id(&path, "workshop")?)
		.await?;
	let options: Vec<ChargeOptionDto> = state.payments.options(&row).into_iter().map(Into::into).collect();
	Ok(HttpResponse::Ok().json(options))
}
