//! `GET /v1/pagos/{id}`

use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use crate::config::AppState;
use crate::error::{AppResult, ErrorBody};
use crate::models::dto::PaymentDto;
use crate::routes::comun::id;
use crate::services::Sesion;

/// Reads a payment; a pending CoDi payment is checked with Pey first.
#[utoipa::path(
	tag = "pagos",
	operation_id = "pagos_ver",
	params(("id" = String, Path, description = "Payment id")),
	responses(
		(status = 200, description = "The payment", body = PaymentDto),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "Not a payment of this tallerista", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[get("/{id}")]
pub(super) async fn ver_pago(
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	sesion: web::ReqData<Sesion>,
	path: web::Path<String>,
) -> AppResult<HttpResponse> {
	let payment = state.payments.get(&conn, sesion.id, id(&path, "payment")?).await?;
	Ok(HttpResponse::Ok().json(PaymentDto::from(payment)))
}
