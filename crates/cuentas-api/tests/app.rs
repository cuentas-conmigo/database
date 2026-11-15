//! The app's routes under `/v1`, over Amanda's tables: login, roster, charges, Pey simulation and webhooks.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::{App, test, web};
use base64::Engine;
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;

use cuentas_api::config::AppState;
use cuentas_api::middleware::auth::ControlAcceso;
use cuentas_api::services::acceso::basic;
use cuentas_api::services::cobros::PaymentService;
use cuentas_api::services::eventos::MemoryEvents;
use cuentas_api::services::pey::SimulatedPey;
use cuentas_api::{routes, services};

const USER: &str = "mariagonzalez";
const PASS: &str = "clave-de-prueba";
const WEBHOOK_SECRET: &str = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";

struct Harness {
	/// Seconds added to the simulated Pey's clock.
	offset: Arc<AtomicI64>,
}

async fn setup() -> (
	Harness,
	impl Service<actix_http::Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error>,
) {
	setup_with(None).await
}

/// Like `setup`, with `PEY_AMOUNT_OVERRIDE` set to `amount_override`.
async fn setup_with(
	amount_override: Option<f64>,
) -> (
	Harness,
	impl Service<actix_http::Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error>,
) {
	let db = services::base_de_prueba().await;
	let siembra = services::Siembra {
		datos: true,
		admin_clave: Some(services::ADMIN_CLAVE_PRUEBAS.into()),
		demo: Some((USER.into(), PASS.into())),
	};
	services::sembrar(&db, &siembra).await.expect("seed");
	services::sembrar(&db, &siembra).await.expect("seed twice");

	let offset = Arc::new(AtomicI64::new(0));
	let clock_offset = offset.clone();
	let clock = Arc::new(move || chrono::Utc::now() + chrono::Duration::seconds(clock_offset.load(Ordering::SeqCst)));
	let api = web::Data::new(AppState {
		payments: PaymentService {
			pey: Arc::new(SimulatedPey::new(Duration::from_secs(4), clock)),
			events: Arc::new(MemoryEvents::default()),
			period: "2026-12".into(),
			amount_override,
		},
		capacity: 12,
		webhook_secret: Some(WEBHOOK_SECRET.into()),
	});
	let app = test::init_service(
		App::new()
			.app_data(web::Data::new(db))
			.app_data(api)
			.wrap(ControlAcceso)
			.configure(routes::config),
	)
	.await;
	(Harness { offset }, app)
}

fn auth() -> (&'static str, String) {
	("Authorization", basic(USER, PASS))
}

async fn call<B: MessageBody>(
	app: &impl Service<actix_http::Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
	request: test::TestRequest,
) -> (StatusCode, Value) {
	let response = test::call_service(app, request.to_request()).await;
	let status = response.status();
	let body = test::read_body(response).await;
	(status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn workshop_id<B: MessageBody>(
	app: &impl Service<actix_http::Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
) -> String {
	let (status, session) = call(
		app,
		test::TestRequest::post()
			.uri("/v1/sesion")
			.set_json(json!({ "usuario": USER, "clave": PASS })),
	)
	.await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(session["name"], "María González");
	assert_eq!(session["role"], "instructor");
	session["workshopId"].as_str().unwrap().to_string()
}

async fn student_id<B: MessageBody>(
	app: &impl Service<actix_http::Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
	workshop: &str,
	name: &str,
) -> String {
	let (_, roster) = call(
		app,
		test::TestRequest::get()
			.uri(&format!("/v1/talleres/{workshop}/alumnos"))
			.insert_header(auth()),
	)
	.await;
	roster
		.as_array()
		.unwrap()
		.iter()
		.find(|entry| entry["student"]["name"] == name)
		.unwrap()["student"]["id"]
		.as_str()
		.unwrap()
		.to_string()
}

#[actix_web::test]
async fn amandas_routes_keep_their_own_rules() {
	let (_, app) = setup().await;

	let page = test::call_service(&app, test::TestRequest::get().uri("/").to_request()).await;
	assert_eq!(page.status(), StatusCode::OK);
	let (status, body) = call(&app, test::TestRequest::get().uri("/v1/alumnos")).await;
	assert_eq!(status, StatusCode::UNAUTHORIZED);
	assert!(body["error"].is_string());
}

#[actix_web::test]
async fn api_needs_the_tallerista_login_except_public_routes() {
	let (_, app) = setup().await;

	let (status, body) = call(&app, test::TestRequest::get().uri("/v1/salud")).await;
	assert_eq!((status, body["peyMode"].as_str()), (StatusCode::OK, Some("simulate")));

	let (status, body) = call(&app, test::TestRequest::get().uri("/v1/talleres/1/alumnos")).await;
	assert_eq!(
		(status, body["code"].as_str()),
		(StatusCode::UNAUTHORIZED, Some("unauthorized"))
	);

	let (status, _) = call(
		&app,
		test::TestRequest::get()
			.uri("/v1/talleres/1/alumnos")
			.insert_header(("Authorization", basic(USER, "otra-clave"))),
	)
	.await;
	assert_eq!(status, StatusCode::UNAUTHORIZED);

	let (status, body) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/sesion")
			.set_json(json!({ "usuario": USER, "clave": "otra-clave" })),
	)
	.await;
	assert_eq!(
		(status, body["code"].as_str()),
		(StatusCode::UNAUTHORIZED, Some("unauthorized"))
	);
}

#[actix_web::test]
async fn workshops_of_other_talleristas_are_not_found() {
	let (_, app) = setup().await;

	// Workshop 1 belongs to Amanda's seed tallerista "Marichuy".
	let (status, body) = call(
		&app,
		test::TestRequest::get().uri("/v1/talleres/1").insert_header(auth()),
	)
	.await;
	assert_eq!(
		(status, body["code"].as_str()),
		(StatusCode::NOT_FOUND, Some("not-found"))
	);
}

#[actix_web::test]
async fn a_qr_payment_turns_paid_and_updates_the_roster() {
	let (harness, app) = setup().await;
	let workshop = workshop_id(&app).await;

	let (status, info) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/talleres/{workshop}"))
			.insert_header(auth()),
	)
	.await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(info["name"], "Taller de tejido");
	assert_eq!(info["schedule"], "Martes y jueves, 10:00");
	assert_eq!(info["monthlyFee"], 130);

	let (_, roster) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/talleres/{workshop}/alumnos"))
			.insert_header(auth()),
	)
	.await;
	let statuses: Vec<(String, String)> = roster
		.as_array()
		.unwrap()
		.iter()
		.map(|entry| {
			(
				entry["student"]["name"].as_str().unwrap().into(),
				entry["status"].as_str().unwrap().into(),
			)
		})
		.collect();
	assert_eq!(
		statuses,
		vec![
			("Rosa Martínez".into(), "pending".into()),
			("Ana López".into(), "paid".into()),
			("Élida Ramírez".into(), "pending".into()),
			("Carmen Ortiz".into(), "pending".into()),
		]
	);

	let (_, options) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/talleres/{workshop}/cobros"))
			.insert_header(auth()),
	)
	.await;
	assert_eq!(
		options[0],
		json!({ "id": "monthly-2026-12", "label": "Diciembre", "detail": "Mensualidad", "amount": 130 })
	);

	let rosa = student_id(&app, &workshop, "Rosa Martínez").await;
	let (status, payment) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/pagos/cobrar")
			.insert_header(auth())
			.set_json(json!({
					"workshopId": workshop, "studentId": rosa, "optionId": "monthly-2026-12",
					"method": "qr", "amount": 130, "concept": "Diciembre"
			})),
	)
	.await;
	assert_eq!(status, StatusCode::CREATED);
	assert_eq!(payment["status"], "pending");
	assert!(payment["qrString"].as_str().unwrap().starts_with("DEMO-"));
	assert!(payment["folio"].as_str().unwrap().starts_with("A-"));
	let id = payment["id"].as_str().unwrap().to_string();

	let (_, again) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/pagos/{id}"))
			.insert_header(auth()),
	)
	.await;
	assert_eq!(again["status"], "pending");
	assert_eq!(again["qrString"], payment["qrString"]);

	harness.offset.store(5, Ordering::SeqCst);
	let (_, paid) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/pagos/{id}"))
			.insert_header(auth()),
	)
	.await;
	assert_eq!(paid["status"], "paid");
	assert!(paid["paidAt"].as_str().unwrap().ends_with("+00:00"));
	assert_eq!(paid["concept"], "Diciembre");

	let (_, roster) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/talleres/{workshop}/alumnos"))
			.insert_header(auth()),
	)
	.await;
	let rosa_entry = roster
		.as_array()
		.unwrap()
		.iter()
		.find(|entry| entry["student"]["name"] == "Rosa Martínez")
		.unwrap();
	assert_eq!(rosa_entry["status"], "paid");
}

#[actix_web::test]
async fn cash_is_paid_at_once_and_a_typed_amount_is_not_the_monthly_fee() {
	let (_, app) = setup().await;
	let workshop = workshop_id(&app).await;
	let carmen = student_id(&app, &workshop, "Carmen Ortiz").await;

	let (status, custom) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/pagos/cobrar")
			.insert_header(auth())
			.set_json(json!({
					"workshopId": workshop, "studentId": carmen, "optionId": "custom",
					"method": "cash", "amount": 75, "concept": "Otro cobro"
			})),
	)
	.await;
	assert_eq!(status, StatusCode::CREATED);
	assert_eq!(
		(custom["status"].as_str(), custom["amount"].as_i64()),
		(Some("paid"), Some(75))
	);
	assert!(custom.get("qrString").is_none());

	let (_, roster) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/talleres/{workshop}/alumnos"))
			.insert_header(auth()),
	)
	.await;
	let entry = roster
		.as_array()
		.unwrap()
		.iter()
		.find(|entry| entry["student"]["name"] == "Carmen Ortiz")
		.unwrap();
	assert_eq!(entry["status"], "pending");

	let (status, body) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/pagos/cobrar")
			.insert_header(auth())
			.set_json(json!({
					"workshopId": workshop, "studentId": carmen, "optionId": "monthly-2027-01",
					"method": "cash", "amount": 130, "concept": "Enero"
			})),
	)
	.await;
	assert_eq!(
		(status, body["code"].as_str()),
		(StatusCode::BAD_REQUEST, Some("bad-request"))
	);
}

fn sign(id: &str, timestamp: i64, body: &[u8]) -> String {
	let key = base64::engine::general_purpose::STANDARD
		.decode(WEBHOOK_SECRET.trim_start_matches("whsec_"))
		.unwrap();
	let mut mac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
	mac.update(format!("{id}.{timestamp}.").as_bytes());
	mac.update(body);
	format!(
		"v1,{}",
		base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
	)
}

#[actix_web::test]
async fn a_signed_pey_webhook_settles_the_payment_once() {
	let (_, app) = setup().await;
	let workshop = workshop_id(&app).await;
	let elida = student_id(&app, &workshop, "Élida Ramírez").await;
	let (_, payment) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/pagos/cobrar")
			.insert_header(auth())
			.set_json(json!({
					"workshopId": workshop, "studentId": elida, "optionId": "monthly-2026-12",
					"method": "qr", "amount": 130, "concept": "Diciembre"
			})),
	)
	.await;
	let id = payment["id"].as_str().unwrap();
	let qr_id = format!("demo-{:07}", id.parse::<i32>().unwrap());
	let body = serde_json::to_vec(&json!({
			"type": "codi.qr.payment_received", "amount": 130, "codiQrId": qr_id,
			"transactionId": "01PEYTXN", "datePaid": chrono::Utc::now().timestamp()
	}))
	.unwrap();
	let now = chrono::Utc::now().timestamp();
	let webhook = |signature: String| {
		test::TestRequest::post()
			.uri("/v1/webhooks/pey")
			.insert_header(("content-type", "application/json"))
			.insert_header(("webhook-id", "msg_1"))
			.insert_header(("webhook-timestamp", now.to_string()))
			.insert_header(("webhook-signature", signature))
			.set_payload(body.clone())
	};

	let (status, _) = call(&app, webhook("v1,Zm9yZ2Vk".into())).await;
	assert_eq!(status, StatusCode::UNAUTHORIZED);

	let (status, first) = call(&app, webhook(sign("msg_1", now, &body))).await;
	assert_eq!((status, first["handled"].as_bool()), (StatusCode::OK, Some(true)));
	let (status, repeat) = call(&app, webhook(sign("msg_1", now, &body))).await;
	assert_eq!((status, repeat["handled"].as_bool()), (StatusCode::OK, Some(false)));

	let (_, paid) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/pagos/{id}"))
			.insert_header(auth()),
	)
	.await;
	assert_eq!(paid["status"], "paid");
}

#[actix_web::test]
async fn without_the_override_amounts_are_the_real_fees() {
	let (_harness, app) = setup().await;
	let workshop = workshop_id(&app).await;
	let rosa = student_id(&app, &workshop, "Rosa Martínez").await;

	let (_, options) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/talleres/{workshop}/cobros"))
			.insert_header(auth()),
	)
	.await;
	let amounts: Vec<_> = options
		.as_array()
		.unwrap()
		.iter()
		.map(|item| item["amount"].clone())
		.collect();
	assert_eq!(amounts, [json!(130), json!(125)]);

	let (status, payment) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/pagos/cobrar")
			.insert_header(auth())
			.set_json(json!({
					"workshopId": workshop, "studentId": rosa, "optionId": "monthly-2026-12",
					"method": "qr", "amount": 130, "concept": "Diciembre"
			})),
	)
	.await;
	assert_eq!(status, StatusCode::CREATED);
	assert_eq!(payment["amount"], 130);
}

#[actix_web::test]
async fn the_amount_override_is_what_every_screen_and_pagos_show() {
	let (_harness, app) = setup_with(Some(1.0)).await;
	let workshop = workshop_id(&app).await;
	let rosa = student_id(&app, &workshop, "Rosa Martínez").await;

	let (_, options) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/talleres/{workshop}/cobros"))
			.insert_header(auth()),
	)
	.await;
	let options = options.as_array().unwrap();
	assert_eq!(options.len(), 2);
	assert!(options.iter().all(|item| item["amount"] == 1), "{options:?}");
	assert_eq!(options[0]["label"], "Diciembre");

	for (option, typed) in [("monthly-2026-12", 130), ("custom", 75)] {
		let (status, payment) = call(
			&app,
			test::TestRequest::post()
				.uri("/v1/pagos/cobrar")
				.insert_header(auth())
				.set_json(json!({
						"workshopId": workshop, "studentId": rosa, "optionId": option,
						"method": "cash", "amount": typed, "concept": "Prueba"
				})),
		)
		.await;
		assert_eq!(status, StatusCode::CREATED, "{payment}");
		assert_eq!(payment["amount"], 1, "{option}");
	}

	let (_, pagos) = call(
		&app,
		test::TestRequest::get().uri("/v1/pagos").insert_header((
			"Authorization",
			basic(services::ADMIN_USUARIO, services::ADMIN_CLAVE_PRUEBAS),
		)),
	)
	.await;
	let montos: Vec<_> = pagos
		.as_array()
		.unwrap()
		.iter()
		.filter(|row| row["alumno_id"].as_i64().map(|id| id.to_string()).as_deref() == Some(rosa.as_str()))
		.map(|row| row["monto_mxn"].clone())
		.collect();
	assert_eq!(montos, [json!(1), json!(1)]);
}

#[actix_web::test]
async fn the_openapi_document_is_not_served_but_lists_the_contract() {
	let (_harness, app) = setup().await;

	let (status, _) = call(&app, test::TestRequest::get().uri("/v1/openapi.json")).await;
	assert_ne!(status, StatusCode::OK);
	let (status, _) = call(&app, test::TestRequest::get().uri("/api/v1/openapi.json")).await;
	assert_ne!(status, StatusCode::OK);

	let doc = cuentas_api::docs::document();
	assert_eq!(doc["info"]["title"], "Cuentas API");
	assert!(doc["paths"]["/v1/pagos/cobrar"]["post"].is_object());
	assert!(doc["paths"]["/v1/talleres/{id}/alumnos"]["get"].is_object());
}
