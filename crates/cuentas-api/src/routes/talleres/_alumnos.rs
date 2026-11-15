//! `GET /v1/talleres/{id}/alumnos`

use actix_web::{HttpResponse, get, web};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::config::AppState;
use crate::error::{AppResult, ErrorBody};
use crate::models::dto::RosterEntryDto;
use crate::models::entities::{alumno, inscripcion};
use crate::routes::comun::id;
use crate::services::Sesion;
use crate::services::{cobros, contactos};

/// Enrolled students with the status of the current month's fee.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_alumnos",
	params(("id" = String, Path, description = "Workshop id")),
	responses(
		(status = 200, description = "Enrolled students", body = [RosterEntryDto]),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "Not a workshop of this tallerista", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[get("/{id}/alumnos")]
pub(super) async fn alumnos_del_taller(
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	sesion: web::ReqData<Sesion>,
	path: web::Path<String>,
) -> AppResult<HttpResponse> {
	let row = state
		.payments
		.workshop(&conn, sesion.id, id(&path, "workshop")?)
		.await?;
	let enrolled = inscripcion::Entity::find()
		.filter(inscripcion::Column::TallerId.eq(row.id))
		.filter(inscripcion::Column::Activa.eq(1))
		.order_by_asc(inscripcion::Column::Id)
		.all(conn.get_ref())
		.await?;
	let paid = cobros::paid_students(&conn, row.id, &state.payments.period).await?;
	let mut seen = Vec::new();
	let mut entries = Vec::new();
	for item in enrolled {
		if seen.contains(&item.alumno_id) {
			continue;
		}
		seen.push(item.alumno_id);
		if let Some(student) = alumno::Entity::find_by_id(item.alumno_id).one(conn.get_ref()).await? {
			let status = if paid.contains(&student.id) { "paid" } else { "pending" };
			entries.push(RosterEntryDto {
				student: contactos::perfil(&conn, student).await?,
				status,
			});
		}
	}
	Ok(HttpResponse::Ok().json(entries))
}
