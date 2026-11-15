//! Errores de las rutas.
//!
//! - [`ApiError`]: el de Amanda, para sus rutas: un estado HTTP y un mensaje para la persona.
//! - [`AppError`]: el de las rutas de la app, `{ code, message }`, la forma que lee la app.

use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use sea_orm::DbErr;

use crate::services::cobros::PaymentError;

#[derive(Debug)]
pub struct ApiError {
	status: StatusCode,
	message: String,
}

impl ApiError {
	pub(crate) fn mensaje(status: StatusCode, message: impl Into<String>) -> Self {
		Self {
			status,
			message: message.into(),
		}
	}

	pub(crate) fn no(nombre: &str, id: i32) -> Self {
		Self::mensaje(StatusCode::NOT_FOUND, format!("No hay {nombre} con id {id}"))
	}

	pub(crate) fn db(error: DbErr) -> Self {
		let texto = error.to_string();
		let (status, message) = if texto.contains("FOREIGN KEY") || texto.contains("foreign key") {
			(
				StatusCode::BAD_REQUEST,
				"Ese id no existe, o el registro todavía tiene inscripciones, pagos o talleres ligados.".to_string(),
			)
		} else if texto.contains("UNIQUE") || texto.contains("duplicate key") {
			(
				StatusCode::BAD_REQUEST,
				"Ya existe un registro con ese dato.".to_string(),
			)
		} else if texto.contains("CHECK") || texto.contains("check constraint") {
			(
				StatusCode::BAD_REQUEST,
				"Algún dato no cumple las reglas de monto, canal, estado o tipo.".to_string(),
			)
		} else if let Some((_, aviso)) = texto.split_once("aviso: ") {
			(StatusCode::BAD_REQUEST, aviso.to_string())
		} else {
			(StatusCode::INTERNAL_SERVER_ERROR, texto)
		};
		Self { status, message }
	}
}

impl std::fmt::Display for ApiError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.message)
	}
}

impl ResponseError for ApiError {
	fn status_code(&self) -> StatusCode {
		self.status
	}

	fn error_response(&self) -> HttpResponse {
		HttpResponse::build(self.status).json(serde_json::json!({ "error": self.message }))
	}
}

/// Body of every error of the app's routes.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ErrorBody {
	/// Stable code the app reads, e.g. `unauthorized`, `not-found`, `not-implemented`.
	#[schema(example = "not-found")]
	pub code: String,
	/// English text for logs, not for people.
	pub message: String,
}

#[derive(Debug)]
pub struct AppError {
	status: StatusCode,
	code: &'static str,
	message: String,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
	pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
		Self {
			status,
			code,
			message: message.into(),
		}
	}

	pub fn not_found(what: &str) -> Self {
		Self::new(StatusCode::NOT_FOUND, "not-found", format!("{what} not found"))
	}

	pub fn unauthorized() -> Self {
		Self::new(
			StatusCode::UNAUTHORIZED,
			"unauthorized",
			"wrong or missing tallerista login",
		)
	}

	pub fn not_implemented(what: &str) -> Self {
		Self::new(
			StatusCode::NOT_IMPLEMENTED,
			"not-implemented",
			format!("{what} is not in the schema yet"),
		)
	}

	pub fn duplicate_phone() -> Self {
		Self::new(
			StatusCode::CONFLICT,
			"duplicate-phone",
			"that WhatsApp is already registered",
		)
	}

	pub fn bad_request(message: impl Into<String>) -> Self {
		Self::new(StatusCode::BAD_REQUEST, "bad-request", message)
	}

	pub fn internal(error: impl std::fmt::Display) -> Self {
		tracing::error!(%error, "app route failure");
		Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", "internal error")
	}
}

impl std::fmt::Display for AppError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}: {}", self.code, self.message)
	}
}

impl ResponseError for AppError {
	fn status_code(&self) -> StatusCode {
		self.status
	}

	fn error_response(&self) -> HttpResponse {
		HttpResponse::build(self.status).json(ErrorBody {
			code: self.code.to_string(),
			message: self.message.clone(),
		})
	}
}

impl From<sea_orm::DbErr> for AppError {
	fn from(error: sea_orm::DbErr) -> Self {
		Self::internal(error)
	}
}

impl From<PaymentError> for AppError {
	fn from(error: PaymentError) -> Self {
		match error {
			PaymentError::NotFound => Self::not_found("payment, workshop or student"),
			PaymentError::Invalid(message) => Self::bad_request(message),
			PaymentError::Pey(error) => {
				tracing::warn!(%error, "pey rejected the charge");
				Self::new(
					StatusCode::BAD_GATEWAY,
					"pey-unavailable",
					"Pey could not create the charge",
				)
			}
			PaymentError::Db(error) => Self::internal(error),
			PaymentError::Events(error) => Self::internal(error),
		}
	}
}
