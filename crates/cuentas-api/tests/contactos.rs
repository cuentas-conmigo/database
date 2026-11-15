//! Phones, helpers, photos, enrollment requests and codes (`migrations/0004_…`) through the app's routes.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::{App, test, web};
use serde_json::{Value, json};

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

fn get(uri: &str) -> test::TestRequest {
	test::TestRequest::get().uri(uri).insert_header(auth())
}

fn post(uri: &str) -> test::TestRequest {
	test::TestRequest::post().uri(uri).insert_header(auth())
}

#[actix_web::test]
async fn the_roster_shows_phones_and_rosas_helper() {
	let (_harness, app) = setup().await;
	let workshop = workshop_id(&app).await;

	let (status, roster) = call(&app, get(&format!("/v1/talleres/{workshop}/alumnos"))).await;

	assert_eq!(status, StatusCode::OK);
	let rosa = roster
		.as_array()
		.unwrap()
		.iter()
		.find(|e| e["student"]["name"] == "Rosa Martínez")
		.unwrap();
	assert_eq!(rosa["student"]["phone"], "5500000001");
	assert_eq!(
		rosa["student"]["helpers"],
		json!([{ "name": "Lucía", "relation": "Hija", "phone": "5500001122" }])
	);
	assert!(rosa["student"].get("photo").is_none());
	assert_eq!(roster.as_array().unwrap().len(), 4);
}

#[actix_web::test]
async fn requests_are_listed_approved_and_removed() {
	let (_harness, app) = setup().await;
	let workshop = workshop_id(&app).await;

	let (status, requests) = call(&app, get(&format!("/v1/talleres/{workshop}/solicitudes"))).await;
	assert_eq!(status, StatusCode::OK);
	let list = requests.as_array().unwrap();
	assert_eq!(list.len(), 2);
	assert_eq!(list[0]["student"]["name"], "Luisa Gómez");
	assert_eq!(list[0]["status"], "pending");
	assert_eq!(list[0]["student"]["helpers"][0]["name"], "Jorge");
	assert_eq!(list[1]["student"]["name"], "Teresa Ruiz");
	assert_eq!(list[1]["status"], "waitlist");
	let luisa = list[0]["student"]["id"].as_str().unwrap().to_string();
	let teresa = list[1]["student"]["id"].as_str().unwrap().to_string();

	let (status, _) = call(&app, post(&format!("/v1/talleres/{workshop}/alumnos/{luisa}/aprobar"))).await;
	assert_eq!(status, StatusCode::NO_CONTENT);
	let (_, roster) = call(&app, get(&format!("/v1/talleres/{workshop}/alumnos"))).await;
	assert_eq!(roster.as_array().unwrap().len(), 5);
	let (status, _) = call(&app, post(&format!("/v1/talleres/{workshop}/alumnos/{luisa}/aprobar"))).await;
	assert_eq!(status, StatusCode::NOT_FOUND);

	let (status, _) = call(&app, post(&format!("/v1/talleres/{workshop}/alumnos/{teresa}/baja"))).await;
	assert_eq!(status, StatusCode::NO_CONTENT);
	let (_, requests) = call(&app, get(&format!("/v1/talleres/{workshop}/solicitudes"))).await;
	assert_eq!(requests, json!([]));
	let (_, roster) = call(&app, get(&format!("/v1/talleres/{workshop}/alumnos"))).await;
	assert_eq!(roster.as_array().unwrap().len(), 5);
}

#[actix_web::test]
async fn the_code_rotates_and_the_old_one_is_gone() {
	let (_harness, app) = setup().await;
	let workshop = workshop_id(&app).await;

	let (status, first) = call(&app, get(&format!("/v1/talleres/{workshop}/codigo"))).await;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(first["code"], "TEJIDO-2026");

	let (status, next) = call(&app, post(&format!("/v1/talleres/{workshop}/codigo"))).await;
	assert_eq!(status, StatusCode::OK);
	let next = next["code"].as_str().unwrap().to_string();
	assert!(next.starts_with("TEJIDO-") && next.len() == 11, "{next}");
	assert_ne!(next, "TEJIDO-2026");
	let (_, again) = call(&app, get(&format!("/v1/talleres/{workshop}/codigo"))).await;
	assert_eq!(again["code"], next.as_str());
}

#[actix_web::test]
async fn enrolling_keeps_phone_helper_and_a_consented_photo_and_refuses_a_known_phone() {
	let (_harness, app) = setup().await;
	let workshop = workshop_id(&app).await;
	let uri = format!("/v1/talleres/{workshop}/alumnos");

	let (status, created) = call(
		&app,
		post(&uri).set_json(json!({
			"name": "Inés Vega", "phone": "55 1234 5678", "helperPhone": "5598765432",
			"photoUri": "file:///foto.jpg", "photoConsent": true
		})),
	)
	.await;
	assert_eq!(status, StatusCode::CREATED);
	assert_eq!(created["phone"], "5512345678");
	assert_eq!(created["helpers"][0]["phone"], "5598765432");
	assert_eq!(created["photo"]["uri"], "file:///foto.jpg");
	assert_eq!(created["photo"]["grantedBy"], "Inés Vega");
	let id = created["id"].as_str().unwrap().to_string();

	let (status, without) = call(
		&app,
		test::TestRequest::delete()
			.uri(&format!("/v1/alumnos/{id}/foto"))
			.insert_header(auth()),
	)
	.await;
	assert_eq!(status, StatusCode::OK);
	assert!(without.get("photo").is_none());

	let (status, error) = call(
		&app,
		post(&uri).set_json(json!({ "name": "Otra", "phone": "5512345678", "photoConsent": false })),
	)
	.await;
	assert_eq!(status, StatusCode::CONFLICT);
	assert_eq!(error["code"], "duplicate-phone");

	let (status, _) = call(
		&app,
		post(&uri).set_json(json!({ "name": "Sin foto", "phone": "123", "photoConsent": false })),
	)
	.await;
	assert_eq!(status, StatusCode::BAD_REQUEST);

	let (status, nophoto) = call(
		&app,
		post(&uri).set_json(
			json!({ "name": "Sin permiso", "phone": "5511112222", "photoUri": "file:///x.jpg", "photoConsent": false }),
		),
	)
	.await;
	assert_eq!(status, StatusCode::CREATED);
	assert!(nophoto.get("photo").is_none());
}

#[actix_web::test]
async fn a_helper_payment_is_a_push_to_the_helper_and_settles() {
	let (harness, app) = setup().await;
	let workshop = workshop_id(&app).await;
	let rosa = student_id(&app, &workshop, "Rosa Martínez").await;
	let ana = student_id(&app, &workshop, "Ana López").await;
	let body = |student: &str| json!({ "workshopId": workshop, "studentId": student, "optionId": "monthly-2026-12", "method": "helper", "amount": 130, "concept": "Diciembre" });

	let (status, payment) = call(&app, post("/v1/pagos/cobrar").set_json(body(&rosa))).await;
	assert_eq!(status, StatusCode::CREATED, "{payment}");
	assert_eq!(payment["method"], "helper");
	assert_eq!(payment["status"], "pending");
	assert!(payment.get("qrString").is_none() || payment["qrString"].is_null());
	let id = payment["id"].as_str().unwrap().to_string();

	harness.offset.store(5, Ordering::SeqCst);
	let (_, settled) = call(&app, get(&format!("/v1/pagos/{id}"))).await;
	assert_eq!(settled["status"], "paid");

	let (status, _) = call(&app, post("/v1/pagos/cobrar").set_json(body(&ana))).await;
	assert_eq!(status, StatusCode::BAD_REQUEST);
}
