//! Charging students: rows in Amanda's `pagos` table, Pey for CoDi, and DynamoDB for what her
//! schema has no column for (the QR to draw, how the payment started).

mod opciones;

use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set};

use crate::models::entities::{pago, taller};
use crate::services::eventos::{ChargeRecord, EventStore, EventsError};
use crate::services::pey::{ChargeRefs, ChargeRequest, PeyClient, PeyError, short_concept};

pub use opciones::{Charge, ChargeKind, ChargeOption, options_for, resolve};

/// How the instructor collects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
	Qr,
	Helper,
	Cash,
}

impl Method {
	pub fn parse(value: &str) -> Option<Self> {
		match value {
			"qr" => Some(Self::Qr),
			"helper" => Some(Self::Helper),
			"cash" => Some(Self::Cash),
			_ => None,
		}
	}

	pub fn as_str(self) -> &'static str {
		match self {
			Self::Qr => "qr",
			Self::Helper => "helper",
			Self::Cash => "cash",
		}
	}

	/// Amanda's `canal`: CoDi charges start from a button, cash is noted in the panel.
	fn canal(self) -> &'static str {
		match self {
			Self::Cash => "panel",
			Self::Qr | Self::Helper => "boton",
		}
	}
}

/// A payment as the app sees it.
#[derive(Clone, Debug)]
pub struct Payment {
	pub row: pago::Model,
	pub charge: Option<ChargeRecord>,
}

impl Payment {
	pub fn is_paid(&self) -> bool {
		self.row.estado == "pagado"
	}

	/// Visible receipt number.
	pub fn folio(&self) -> String {
		format!("A-{:04}", self.row.id)
	}

	pub fn paid_at(&self) -> Option<String> {
		self.row.pagado_en.as_deref().map(iso)
	}
}

#[derive(Debug)]
pub enum PaymentError {
	NotFound,
	Invalid(&'static str),
	Pey(PeyError),
	Db(DbErr),
	Events(EventsError),
}

impl From<DbErr> for PaymentError {
	fn from(error: DbErr) -> Self {
		Self::Db(error)
	}
}

impl From<EventsError> for PaymentError {
	fn from(error: EventsError) -> Self {
		Self::Events(error)
	}
}

/// What the app asks to start.
pub struct NewPayment {
	pub tallerista_id: i32,
	pub workshop_id: i32,
	pub student_id: i32,
	pub option_id: String,
	pub method: Method,
	pub amount: f64,
}

pub struct PaymentService {
	pub pey: Arc<dyn PeyClient>,
	pub events: Arc<dyn EventStore>,
	pub period: String,
	pub amount_override: Option<f64>,
}

/// A date as Amanda's tables keep it (`yyyy-mm-dd hh:mm:ss`, UTC, see `ahora_texto()`) as RFC 3339.
pub fn iso(texto: &str) -> String {
	NaiveDateTime::parse_from_str(texto, "%Y-%m-%d %H:%M:%S")
		.map(|time| time.and_utc().to_rfc3339())
		.unwrap_or_else(|_| texto.to_string())
}

fn fecha_texto(time: DateTime<Utc>) -> String {
	time.format("%Y-%m-%d %H:%M:%S").to_string()
}

impl PaymentService {
	/// What is really charged: `PEY_AMOUNT_OVERRIDE` when it is set (team tests with real money), else `amount`.
	pub fn charged(&self, amount: i32) -> i32 {
		match self.amount_override {
			Some(value) => (value.ceil() as i32).max(1),
			None => amount,
		}
	}

	/// The workshop's charge options with the amounts that will really be charged.
	pub fn options(&self, workshop: &taller::Model) -> Vec<ChargeOption> {
		options_for(workshop, &self.period)
			.into_iter()
			.map(|mut option| {
				option.amount = self.charged(option.amount);
				option
			})
			.collect()
	}

	/// Workshop owned by this tallerista.
	pub async fn workshop(
		&self,
		conn: &DatabaseConnection,
		tallerista_id: i32,
		workshop_id: i32,
	) -> Result<taller::Model, PaymentError> {
		taller::Entity::find_by_id(workshop_id)
			.one(conn)
			.await?
			.filter(|row| row.tallerista_id == tallerista_id && row.activo == 1)
			.ok_or(PaymentError::NotFound)
	}

	pub async fn start(&self, conn: &DatabaseConnection, input: NewPayment) -> Result<Payment, PaymentError> {
		let workshop = self.workshop(conn, input.tallerista_id, input.workshop_id).await?;
		crate::models::entities::alumno::Entity::find_by_id(input.student_id)
			.one(conn)
			.await?
			.ok_or(PaymentError::NotFound)?;
		let mut charge = resolve(&workshop, &self.period, &input.option_id, input.amount)
			.ok_or(PaymentError::Invalid("unknown charge option"))?;
		charge.amount = self.charged(charge.amount);
		if input.method == Method::Helper {
			return Err(PaymentError::Invalid("this student has no helper registered"));
		}

		let now = Utc::now();
		let cash = input.method == Method::Cash;
		let (taller_id, tallerista_id) = match charge.kind {
			ChargeKind::Annual => (None, None),
			ChargeKind::Monthly | ChargeKind::Custom => (Some(workshop.id), Some(workshop.tallerista_id)),
		};
		let fila = pago::ActiveModel {
			alumno_id: Set(input.student_id),
			taller_id: Set(taller_id),
			tallerista_id: Set(tallerista_id),
			tipo: Set(charge.tipo().to_string()),
			canal: Set(input.method.canal().to_string()),
			concepto: Set(charge.label.clone()),
			monto_mxn: Set(charge.amount),
			num_ref: Set(format!("pendiente-{}", now.timestamp_nanos_opt().unwrap_or_default())),
			estado: Set("pendiente".to_string()),
			periodo: Set(charge.periodo.clone()),
			..Default::default()
		}
		.insert(conn)
		.await?;

		// Pey's `numRef` is numeric with at most 7 digits; the row id fits and is unique.
		let mut cambio: pago::ActiveModel = fila.into();
		let id = *cambio.id.as_ref();
		cambio.num_ref = Set(format!("{id:07}"));
		if cash {
			cambio.estado = Set("pagado".to_string());
			cambio.transaction_id = Set(Some(format!("efectivo-{id}")));
			cambio.pagado_en = Set(Some(fecha_texto(now)));
		}
		let mut fila = cambio.update(conn).await?;

		let mut record = ChargeRecord {
			pago_id: id,
			workshop_id: workshop.id,
			option_id: input.option_id.clone(),
			method: input.method.as_str().to_string(),
			created_at: now.to_rfc3339(),
			qr_string: None,
		};
		if input.method == Method::Qr {
			let request = ChargeRequest {
				payment_id: id.to_string(),
				amount: f64::from(charge.amount),
				concept: short_concept(&format!("{} {}", charge.label, workshop.nombre)),
				num_ref: format!("{id:07}"),
				created_at: now,
			};
			match self.pey.create_qr(&request).await {
				Ok(qr) => {
					let mut cambio: pago::ActiveModel = fila.into();
					cambio.codi_qr_id = Set(Some(qr.qr_id));
					fila = cambio.update(conn).await?;
					record.qr_string = Some(qr.qr_string);
				}
				Err(error) => {
					let mut cambio: pago::ActiveModel = fila.into();
					cambio.estado = Set("fallido".to_string());
					cambio.update(conn).await?;
					return Err(PaymentError::Pey(error));
				}
			}
		}
		self.events.save_charge(&record).await?;
		Ok(Payment {
			row: fila,
			charge: Some(record),
		})
	}

	/// Reads a payment of this tallerista; a pending CoDi payment is checked with Pey first.
	pub async fn get(
		&self,
		conn: &DatabaseConnection,
		tallerista_id: i32,
		pago_id: i32,
	) -> Result<Payment, PaymentError> {
		let row = pago::Entity::find_by_id(pago_id)
			.one(conn)
			.await?
			.ok_or(PaymentError::NotFound)?;
		let charge = self.events.charge(pago_id).await?;
		let workshop_id = row.taller_id.or(charge.as_ref().map(|item| item.workshop_id));
		let Some(workshop_id) = workshop_id else {
			return Err(PaymentError::NotFound);
		};
		self.workshop(conn, tallerista_id, workshop_id).await?;
		let payment = Payment { row, charge };
		if payment.is_paid() || payment.row.estado == "fallido" {
			return Ok(payment);
		}
		let created_at = payment
			.charge
			.as_ref()
			.and_then(|item| DateTime::parse_from_rfc3339(&item.created_at).ok())
			.map(|time| time.with_timezone(&Utc));
		let refs = ChargeRefs {
			payment_id: &payment.row.id.to_string(),
			qr_id: payment.row.codi_qr_id.as_deref(),
			transaction_id: payment.row.transaction_id.as_deref(),
			created_at,
		};
		// A Pey outage must not break polling; the webhook still settles the payment.
		let paid_at = self.pey.paid_at(&refs).await.unwrap_or(None);
		match paid_at {
			Some(paid_at) => {
				let transaction = payment
					.row
					.transaction_id
					.clone()
					.or_else(|| payment.row.codi_qr_id.clone())
					.unwrap_or_else(|| format!("pey-{}", payment.row.id));
				let row = mark_paid(conn, payment.row, &transaction, "codi.qr.payment_received", paid_at).await?;
				Ok(Payment {
					row,
					charge: payment.charge,
				})
			}
			None => Ok(payment),
		}
	}

	/// Settles the row a Pey webhook points to; returns `false` when no row matches.
	pub async fn settle_from_webhook(
		&self,
		conn: &DatabaseConnection,
		event: &crate::services::pey::webhook::PaymentEvent,
	) -> Result<bool, PaymentError> {
		let mut found = None;
		if let Some(qr_id) = &event.codi_qr_id {
			found = pago::Entity::find()
				.filter(pago::Column::CodiQrId.eq(qr_id.as_str()))
				.one(conn)
				.await?;
		}
		if found.is_none()
			&& let Some(transaction_id) = &event.transaction_id
		{
			found = pago::Entity::find()
				.filter(pago::Column::TransactionId.eq(transaction_id.as_str()))
				.one(conn)
				.await?;
		}
		if found.is_none()
			&& let Some(id) = event.payment_id().and_then(|id| id.parse::<i32>().ok())
		{
			found = pago::Entity::find_by_id(id).one(conn).await?;
		}
		let Some(row) = found else {
			return Ok(false);
		};
		if row.estado == "pagado" {
			return Ok(true);
		}
		let paid_at = event
			.date_paid
			.and_then(|seconds| DateTime::from_timestamp(seconds, 0))
			.unwrap_or_else(Utc::now);
		let transaction = event
			.transaction_id
			.clone()
			.or_else(|| row.codi_qr_id.clone())
			.unwrap_or_else(|| format!("pey-{}", row.id));
		mark_paid(conn, row, &transaction, tipo_evento(&event.kind), paid_at).await?;
		Ok(true)
	}
}

/// Amanda's `tipo_evento` allows `codi.push.payment_received`; Pey calls it `codi.pn.payment_received`.
fn tipo_evento(kind: &str) -> &'static str {
	if kind == "codi.pn.payment_received" {
		"codi.push.payment_received"
	} else {
		"codi.qr.payment_received"
	}
}

async fn mark_paid(
	conn: &DatabaseConnection,
	row: pago::Model,
	transaction_id: &str,
	tipo_evento: &str,
	paid_at: DateTime<Utc>,
) -> Result<pago::Model, DbErr> {
	let mut cambio: pago::ActiveModel = row.into();
	cambio.estado = Set("pagado".to_string());
	if cambio.transaction_id.as_ref().is_none() {
		cambio.transaction_id = Set(Some(transaction_id.to_string()));
	}
	cambio.tipo_evento = Set(Some(tipo_evento.to_string()));
	cambio.pagado_en = Set(Some(fecha_texto(paid_at)));
	cambio.update(conn).await
}

/// Students whose monthly fee for the period is paid in this workshop.
pub async fn paid_students(conn: &DatabaseConnection, workshop_id: i32, period: &str) -> Result<Vec<i32>, DbErr> {
	let filas = pago::Entity::find()
		.filter(pago::Column::TallerId.eq(workshop_id))
		.filter(pago::Column::Tipo.eq("mensualidad"))
		.filter(pago::Column::Periodo.eq(period))
		.filter(pago::Column::Estado.eq("pagado"))
		.all(conn)
		.await?;
	Ok(filas.into_iter().map(|fila| fila.alumno_id).collect())
}
