//! `GET /v1/talleres/{id}`

use actix_web::{HttpResponse, get, web};
use sea_orm::{DatabaseConnection, EntityTrait};

use super::horario_texto;
use crate::config::AppState;
use crate::error::{AppResult, ErrorBody};
use crate::models::dto::WorkshopDto;
use crate::models::entities::tallerista;
use crate::routes::comun::id;
use crate::services::Sesion;

/// One of the tallerista's workshops.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_ver",
	params(("id" = String, Path, description = "Workshop id")),
	responses(
		(status = 200, description = "The workshop", body = WorkshopDto),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "Not a workshop of this tallerista", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[get("/{id}")]
pub(super) async fn ver_taller(
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	sesion: web::ReqData<Sesion>,
	path: web::Path<String>,
) -> AppResult<HttpResponse> {
	let row = state
		.payments
		.workshop(&conn, sesion.id, id(&path, "workshop")?)
		.await?;
	let instructor = tallerista::Entity::find_by_id(row.tallerista_id)
		.one(conn.get_ref())
		.await?
		.map(|row| row.nombre)
		.unwrap_or_default();
	Ok(HttpResponse::Ok().json(WorkshopDto {
		id: row.id.to_string(),
		schedule: horario_texto(&conn, row.id).await?,
		name: row.nombre,
		instructor,
		monthly_fee: state.payments.charged(row.cuota_mensual_mxn),
		capacity: state.capacity,
	}))
}
