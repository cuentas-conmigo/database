//! Live mode against a fake Pey server: request shape, Bearer key, MXN 1 override and the status
//! read that settles a payment when no webhook arrives.

use std::sync::{Arc, Mutex};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};

use cuentas_api::services;
use cuentas_api::services::cobros::{Method, NewPayment, PaymentService};
use cuentas_api::services::eventos::MemoryEvents;
use cuentas_api::services::pey::{LivePey, PeyCredentials};

#[derive(Default)]
struct Seen {
	qr_body: Option<Value>,
	authorization: Option<String>,
	status_reads: usize,
}

async fn fake_pey(seen: Arc<Mutex<Seen>>) -> String {
	let server = HttpServer::new(move || {
		let seen = seen.clone();
		App::new()
			.app_data(web::Data::new(seen))
			.route(
				"/codi/qr",
				web::post().to(
					|req: HttpRequest, body: web::Json<Value>, seen: web::Data<Arc<Mutex<Seen>>>| async move {
						let mut seen = seen.lock().unwrap();
						seen.qr_body = Some(body.into_inner());
						seen.authorization = req
							.headers()
							.get("authorization")
							.and_then(|value| value.to_str().ok())
							.map(str::to_string);
						HttpResponse::Ok().json(json!({ "id": "35a13c183a", "qrString": "00020101021226850014MX.ORG.CODI" }))
					},
				),
			)
			.route(
				"/transaction/codi/qr/{id}",
				web::get().to(|seen: web::Data<Arc<Mutex<Seen>>>| async move {
					seen.lock().unwrap().status_reads += 1;
					HttpResponse::Ok().json(json!({ "items": [{ "status": "PAID", "datePaid": 1795000000 }] }))
				}),
			)
	})
	.workers(1)
	.bind(("127.0.0.1", 0))
	.unwrap();
	let address = server.addrs()[0];
	actix_web::rt::spawn(server.run());
	format!("http://{address}")
}

#[actix_web::test]
async fn live_mode_creates_a_real_shaped_charge_and_reads_its_status() {
	let seen = Arc::new(Mutex::new(Seen::default()));
	let base_url = fake_pey(seen.clone()).await;
	let conn = services::base_de_prueba().await;
	let siembra = services::Siembra {
		datos: true,
		admin_clave: None,
		demo: Some(("mariagonzalez".into(), "clave-de-prueba".into())),
	};
	services::sembrar(&conn, &siembra).await.unwrap();
	let ids = conn
		.query_one_raw(Statement::from_string(
			DbBackend::Postgres,
			"SELECT t.id AS tallerista, w.id AS taller, a.id AS alumno
             FROM talleristas t JOIN talleres w ON w.tallerista_id = t.id
             JOIN inscripciones_taller i ON i.taller_id = w.id JOIN alumnos a ON a.id = i.alumno_id
             WHERE t.usuario = 'mariagonzalez' AND a.nombre = 'Rosa Martínez'",
		))
		.await
		.unwrap()
		.unwrap();

	let credentials =
		PeyCredentials::parse(r#"{"apiKey":"test-key","webhookSecret":"whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw"}"#).unwrap();
	let service = PaymentService {
		pey: Arc::new(LivePey::new(base_url, credentials).unwrap()),
		events: Arc::new(MemoryEvents::default()),
		period: "2026-12".into(),
		amount_override: Some(1.0),
	};
	let tallerista: i32 = ids.try_get("", "tallerista").unwrap();
	let payment = service
		.start(
			&conn,
			NewPayment {
				tallerista_id: tallerista,
				workshop_id: ids.try_get("", "taller").unwrap(),
				student_id: ids.try_get("", "alumno").unwrap(),
				option_id: "monthly-2026-12".into(),
				method: Method::Qr,
				amount: 130.0,
			},
		)
		.await
		.unwrap();

	assert!(!payment.is_paid());
	// `PEY_AMOUNT_OVERRIDE` is what is really charged, so it is also what `pagos` records.
	assert_eq!(payment.row.monto_mxn, 1);
	assert_eq!(payment.row.codi_qr_id.as_deref(), Some("35a13c183a"));
	assert_eq!(
		payment.charge.as_ref().unwrap().qr_string.as_deref(),
		Some("00020101021226850014MX.ORG.CODI")
	);
	{
		let seen = seen.lock().unwrap();
		assert_eq!(seen.authorization.as_deref(), Some("Bearer test-key"));
		let body = seen.qr_body.as_ref().unwrap();
		assert_eq!(body["amount"], 1.0);
		assert_eq!(body["numRef"], format!("{:07}", payment.row.id));
		assert_eq!(body["metadata"]["pagoId"], payment.row.id.to_string());
	}

	let paid = service.get(&conn, tallerista, payment.row.id).await.unwrap();
	assert!(paid.is_paid());
	assert_eq!(paid.row.pagado_en.as_deref(), Some("2026-11-18 11:06:40"));
	assert_eq!(seen.lock().unwrap().status_reads, 1);
}
