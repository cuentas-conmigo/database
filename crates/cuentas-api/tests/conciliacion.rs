//! The background sweep settles a CoDi charge that Pey reports paid when no webhook arrives,
//! and leaves alone rows Pey never knew about.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use actix_web::{App, HttpResponse, HttpServer, web};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbBackend, EntityTrait, Set, Statement};
use serde_json::json;

use cuentas_api::models::entities::pago;
use cuentas_api::services;
use cuentas_api::services::cobros::{Method, NewPayment, PaymentService, Reconciler};
use cuentas_api::services::eventos::MemoryEvents;
use cuentas_api::services::pey::{LivePey, PeyCredentials};

#[derive(Default)]
struct FakePey {
	paid: AtomicBool,
	status_reads: AtomicUsize,
}

async fn fake_pey(pey: Arc<FakePey>) -> String {
	let server = HttpServer::new(move || {
		App::new()
			.app_data(web::Data::from(pey.clone()))
			.route(
				"/codi/qr",
				web::post()
					.to(|| async { HttpResponse::Created().json(json!({ "id": "a1b2c3d4e5", "qrString": "0002010102" })) }),
			)
			.route(
				"/transaction/codi/qr/{id}",
				web::get().to(|pey: web::Data<FakePey>| async move {
					pey.status_reads.fetch_add(1, Ordering::SeqCst);
					let items = if pey.paid.load(Ordering::SeqCst) {
						json!([{ "id": "01KTXN0000000000000000PAID", "status": "PAID", "datePaid": 1795000000 }])
					} else {
						json!([])
					};
					HttpResponse::Ok().json(json!({ "items": items, "nextToken": null }))
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
async fn the_sweep_settles_a_qr_charge_paid_without_a_webhook() {
	let pey = Arc::new(FakePey::default());
	let base_url = fake_pey(pey.clone()).await;
	let conn = services::base_de_prueba().await;
	let ids = conn
		.query_one_raw(Statement::from_string(
			DbBackend::Postgres,
			"SELECT w.tallerista_id AS tallerista, w.id AS taller, i.alumno_id AS alumno
             FROM talleres w JOIN inscripciones_taller i ON i.taller_id = w.id
             ORDER BY w.id, i.alumno_id LIMIT 1",
		))
		.await
		.unwrap()
		.unwrap();
	let credentials = PeyCredentials::parse("test-key").unwrap();
	let service = PaymentService {
		pey: Arc::new(LivePey::new(base_url, credentials).unwrap()),
		events: Arc::new(MemoryEvents::default()),
		period: "2026-12".into(),
		amount_override: Some(1.0),
		approval_required: false,
	};
	let student: i32 = ids.try_get("", "alumno").unwrap();
	let charge = service
		.start(
			&conn,
			NewPayment {
				tallerista_id: ids.try_get("", "tallerista").unwrap(),
				workshop_id: ids.try_get("", "taller").unwrap(),
				student_id: student,
				option_id: "monthly-2026-12".into(),
				method: Method::Qr,
				amount: 130.0,
			},
		)
		.await
		.unwrap();
	// A row from Amanda's admin pages: Pey never saw it, so the sweep must not ask about it.
	let manual = pago::ActiveModel {
		alumno_id: Set(student),
		tipo: Set("inscripcion_anual".into()),
		canal: Set("boton".into()),
		concepto: Set("Inscripción anual 2026".into()),
		monto_mxn: Set(1),
		num_ref: Set("manual-1".into()),
		codi_qr_id: Set(Some("qr-manual-1".into())),
		estado: Set("pendiente".into()),
		periodo: Set("2026".into()),
		..Default::default()
	}
	.insert(&conn)
	.await
	.unwrap();

	let mut reconciler = Reconciler::default();
	let now = Utc::now();
	let pending = reconciler.sweep(&service, &conn, now).await.unwrap();
	assert_eq!((pending.checked, pending.settled.len()), (1, 0));

	pey.paid.store(true, Ordering::SeqCst);
	// LivePey asks about one payment at most every 5 s, so the next sweep is a minute later in
	// the schedule and after that pause in real time.
	actix_web::rt::time::sleep(std::time::Duration::from_millis(5100)).await;
	let paid = reconciler
		.sweep(&service, &conn, now + Duration::minutes(1))
		.await
		.unwrap();
	assert_eq!(paid.settled, vec![charge.row.id]);
	assert_eq!(pey.status_reads.load(Ordering::SeqCst), 2);

	let row = pago::Entity::find_by_id(charge.row.id)
		.one(&conn)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(row.estado, "pagado");
	assert_eq!(row.transaction_id.as_deref(), Some("01KTXN0000000000000000PAID"));
	assert_eq!(row.tipo_evento.as_deref(), Some("codi.qr.payment_received"));
	assert_eq!(row.pagado_en.as_deref(), Some("2026-11-18 11:06:40"));
	let manual = pago::Entity::find_by_id(manual.id).one(&conn).await.unwrap().unwrap();
	assert_eq!(manual.estado, "pendiente");

	let after = reconciler
		.sweep(&service, &conn, now + Duration::minutes(2))
		.await
		.unwrap();
	assert_eq!(after, Default::default());
}
