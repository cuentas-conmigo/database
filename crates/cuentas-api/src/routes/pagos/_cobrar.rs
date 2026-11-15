//! `POST /v1/pagos/cobrar`

use actix_web::{HttpResponse, http::StatusCode, post, web};
use sea_orm::DatabaseConnection;

use crate::config::AppState;
use crate::error::{AppError, AppResult, ErrorBody};
use crate::models::dto::{NewPaymentBody, PaymentDto};
use crate::routes::comun::id;
use crate::services::Sesion;
use crate::services::cobros::{self, NewPayment};

/// Creates a payment: a CoDi QR with Pey, or cash already paid.
#[utoipa::path(
	tag = "pagos",
	operation_id = "pagos_cobrar",
	request_body = NewPaymentBody,
	responses(
		(status = 201, description = "The payment; `qrString` holds the CoDi QR", body = PaymentDto),
		(status = 400, description = "Unknown option, method or amount", body = ErrorBody),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "Workshop or student not found", body = ErrorBody),
		(status = 502, description = "Pey could not create the charge", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[post("/cobrar")]
pub(super) async fn cobrar(
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	sesion: web::ReqData<Sesion>,
	body: web::Json<NewPaymentBody>,
) -> AppResult<HttpResponse> {
	let body = body.into_inner();
	let method =
		cobros::Method::parse(&body.method).ok_or_else(|| AppError::bad_request("method must be qr, helper or cash"))?;
	let payment = state
		.payments
		.start(
			&conn,
			NewPayment {
				tallerista_id: sesion.id,
				workshop_id: id(&body.workshop_id, "workshop")?,
				student_id: id(&body.student_id, "student")?,
				option_id: body.option_id,
				method,
				amount: body.amount,
			},
		)
		.await?;
	Ok(HttpResponse::build(StatusCode::CREATED).json(PaymentDto::from(payment)))
}
