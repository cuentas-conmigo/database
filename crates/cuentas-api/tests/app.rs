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
	setup_with(None, false).await
}

/// Like `setup`, with `PEY_AMOUNT_OVERRIDE` set to `amount_override`, and with CoDi charges only for
/// approved accounts when `approval_required` (as with Pey live).
async fn setup_with(
	amount_override: Option<f64>,
	approval_required: bool,
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
			approval_required,
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
	let (_harness, app) = setup_with(Some(1.0), false).await;
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
async fn only_approved_accounts_start_codi_charges_when_pey_is_live() {
	let (_harness, app) = setup_with(Some(1.0), true).await;
	let workshop = workshop_id(&app).await;
	let rosa = student_id(&app, &workshop, "Rosa Martínez").await;
	let admin = (
		"Authorization",
		basic(services::ADMIN_USUARIO, services::ADMIN_CLAVE_PRUEBAS),
	);
	let charge = |method: &str| {
		test::TestRequest::post()
			.uri("/v1/pagos/cobrar")
			.insert_header(auth())
			.set_json(json!({
					"workshopId": workshop, "studentId": rosa, "optionId": "monthly-2026-12",
					"method": method, "amount": 130
			}))
	};

	let (_, accounts) = call(
		&app,
		test::TestRequest::get()
			.uri("/v1/talleristas/cobros-reales")
			.insert_header(admin.clone()),
	)
	.await;
	let maria = accounts
		.as_array()
		.unwrap_or_else(|| panic!("accounts: {accounts}"))
		.iter()
		.find(|account| account["usuario"] == USER)
		.unwrap()
		.clone();
	assert!(
		maria["aprobadoEn"].is_string(),
		"the seeded demo account starts approved"
	);
	let maria = maria["id"].as_i64().unwrap();
	let (status, _) = call(&app, charge("qr")).await;
	assert_eq!(status, StatusCode::CREATED);

	let (status, _) = call(
		&app,
		test::TestRequest::delete()
			.uri(&format!("/v1/talleristas/{maria}/cobros-reales"))
			.insert_header(admin.clone()),
	)
	.await;
	assert_eq!(status, StatusCode::OK);
	let (status, error) = call(&app, charge("qr")).await;
	assert_eq!(status, StatusCode::FORBIDDEN);
	assert_eq!(error["code"], "charges-not-approved");
	let (status, _) = call(&app, charge("helper")).await;
	assert_eq!(status, StatusCode::FORBIDDEN);
	let (status, cash) = call(&app, charge("cash")).await;
	assert_eq!(status, StatusCode::CREATED, "cash needs no approval");
	assert_eq!(cash["status"], "paid");

	let (status, _) = call(
		&app,
		test::TestRequest::put()
			.uri(&format!("/v1/talleristas/{maria}/cobros-reales"))
			.insert_header(("Authorization", basic(USER, PASS))),
	)
	.await;
	assert_eq!(status, StatusCode::UNAUTHORIZED, "only the admin approves");
	let (status, _) = call(
		&app,
		test::TestRequest::put()
			.uri("/v1/talleristas/999999/cobros-reales")
			.insert_header(admin.clone()),
	)
	.await;
	assert_eq!(status, StatusCode::NOT_FOUND);
	let (status, _) = call(
		&app,
		test::TestRequest::put()
			.uri(&format!("/v1/talleristas/{maria}/cobros-reales"))
			.insert_header(admin),
	)
	.await;
	assert_eq!(status, StatusCode::OK);
	let (status, _) = call(&app, charge("qr")).await;
	assert_eq!(status, StatusCode::CREATED);
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

fn new_account(usuario: &str) -> Value {
	json!({
		"usuario": usuario, "clave": "barro-2026", "name": "Lupe Hernández",
		"trade": "cerámica", "workshop": "Taller de barro", "monthlyFee": 150
	})
}

#[actix_web::test]
async fn the_app_creates_an_account_with_its_first_workshop() {
	let (_harness, app) = setup().await;

	let (status, session) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/cuenta")
			.set_json(new_account("lupita")),
	)
	.await;
	assert_eq!(status, StatusCode::CREATED, "{session}");
	assert_eq!(session["name"], "Lupe Hernández");
	assert_eq!(session["role"], "instructor");
	let workshop = session["workshopId"].as_str().unwrap().to_string();

	let (status, again) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/sesion")
			.set_json(json!({ "usuario": "lupita", "clave": "barro-2026" })),
	)
	.await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(again["workshopId"], workshop.as_str());
	let (status, details) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/talleres/{workshop}"))
			.insert_header(("Authorization", basic("lupita", "barro-2026"))),
	)
	.await;
	assert_eq!(status, StatusCode::OK, "{details}");
	assert_eq!(details["name"], "Taller de barro");
	assert_eq!(details["monthlyFee"], 150);

	let (status, taken) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/cuenta")
			.set_json(new_account("Lupita")),
	)
	.await;
	assert_eq!(status, StatusCode::CONFLICT);
	assert_eq!(taken["code"], "username-taken");

	let mut short = new_account("otra");
	short["clave"] = json!("123");
	let (status, invalid) = call(&app, test::TestRequest::post().uri("/v1/cuenta").set_json(short)).await;
	assert_eq!(status, StatusCode::BAD_REQUEST);
	assert!(invalid["message"].as_str().unwrap().contains("contraseña"), "{invalid}");
	let mut free = new_account("otra");
	free["monthlyFee"] = json!(0);
	let (status, _) = call(&app, test::TestRequest::post().uri("/v1/cuenta").set_json(free)).await;
	assert_eq!(status, StatusCode::BAD_REQUEST);
	let (status, _) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/sesion")
			.set_json(json!({ "usuario": "otra", "clave": "barro-2026" })),
	)
	.await;
	assert_eq!(
		status,
		StatusCode::UNAUTHORIZED,
		"a refused sign-up leaves nothing behind"
	);
}

#[actix_web::test]
async fn deleting_an_account_removes_its_data_and_only_students_who_were_only_theirs() {
	let (_harness, app) = setup().await;
	let admin = (
		"Authorization",
		basic(services::ADMIN_USUARIO, services::ADMIN_CLAVE_PRUEBAS),
	);
	let lupita = ("Authorization", basic("lupita", "barro-2026"));
	let (_, session) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/cuenta")
			.set_json(new_account("lupita")),
	)
	.await;
	let workshop = session["workshopId"].as_str().unwrap().to_string();

	let (status, only_hers) = call(
		&app,
		test::TestRequest::post()
			.uri(&format!("/v1/talleres/{workshop}/alumnos"))
			.insert_header(lupita.clone())
			.set_json(json!({ "name": "Teresa Solo", "phone": "5512340001", "helperPhone": "5512340002" })),
	)
	.await;
	assert_eq!(status, StatusCode::CREATED, "{only_hers}");
	let (status, _) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/pagos/cobrar")
			.insert_header(lupita.clone())
			.set_json(json!({
					"workshopId": workshop, "studentId": only_hers["id"], "optionId": "monthly-2026-12",
					"method": "cash", "amount": 150
			})),
	)
	.await;
	assert_eq!(status, StatusCode::CREATED);
	let demo_workshop = workshop_id(&app).await;
	let rosa = student_id(&app, &demo_workshop, "Rosa Martínez").await;
	let (status, shared) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/inscripciones")
			.insert_header(admin.clone())
			.set_json(json!({ "alumno_id": rosa.parse::<i32>().unwrap(), "taller_id": workshop.parse::<i32>().unwrap() })),
	)
	.await;
	assert!(status.is_success(), "{status} {shared}");

	let (status, _) = call(
		&app,
		test::TestRequest::delete()
			.uri("/v1/cuenta")
			.insert_header(("Authorization", basic(USER, "mal"))),
	)
	.await;
	assert_eq!(status, StatusCode::UNAUTHORIZED);
	let (status, _) = call(
		&app,
		test::TestRequest::delete()
			.uri("/v1/cuenta")
			.insert_header(lupita.clone()),
	)
	.await;
	assert_eq!(status, StatusCode::NO_CONTENT);

	let (status, _) = call(
		&app,
		test::TestRequest::post()
			.uri("/v1/sesion")
			.set_json(json!({ "usuario": "lupita", "clave": "barro-2026" })),
	)
	.await;
	assert_eq!(status, StatusCode::UNAUTHORIZED);
	let (_, students) = call(&app, test::TestRequest::get().uri("/v1/alumnos").insert_header(admin)).await;
	let names: Vec<&str> = students
		.as_array()
		.unwrap_or_else(|| panic!("students: {students}"))
		.iter()
		.filter_map(|student| student["nombre"].as_str())
		.collect();
	assert!(!names.contains(&"Teresa Solo"), "{names:?}");
	assert!(names.contains(&"Rosa Martínez"), "Rosa is also in María's workshop");
	let (_, roster) = call(
		&app,
		test::TestRequest::get()
			.uri(&format!("/v1/talleres/{demo_workshop}/alumnos"))
			.insert_header(auth()),
	)
	.await;
	assert!(
		roster
			.as_array()
			.unwrap()
			.iter()
			.any(|entry| entry["student"]["name"] == "Rosa Martínez")
	);
}
