//! `GET /v1/salud`

use actix_web::{HttpResponse, get, web};

use crate::config::AppState;
use crate::models::dto::HealthDto;

/// Answers without a login, also for load balancers.
#[utoipa::path(
	tag = "salud",
	operation_id = "salud_ver",
	responses((status = 200, description = "The API is up", body = HealthDto)),
)]
#[get("")]
pub(super) async fn salud(state: web::Data<AppState>) -> HttpResponse {
	let mode = if state.payments.pey.is_live() {
		"live"
	} else {
		"simulate"
	};
	HttpResponse::Ok().json(HealthDto {
		ok: true,
		pey_mode: mode,
	})
}
