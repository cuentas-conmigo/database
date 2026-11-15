//! Builds what the server needs from the configuration: database, Pey and the events store.

use std::sync::Arc;

use actix_cors::Cors;
use sea_orm::DatabaseConnection;

use crate::config::{AppState, Config};
use crate::services::cobros::PaymentService;
use crate::services::eventos::{DynamoEvents, EventStore, MemoryEvents};
use crate::services::pey::{LivePey, PeyClient, PeyCredentials, SimulatedPey};

/// Everything shared by the HTTP workers.
pub struct Shared {
	pub db: DatabaseConnection,
	pub api: AppState,
}

#[derive(Debug)]
pub struct StartError(pub String);

impl std::fmt::Display for StartError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

impl std::error::Error for StartError {}

pub async fn build(config: &Config) -> Result<Shared, StartError> {
	let db = crate::services::conectar(&config.database_url)
		.await
		.map_err(|error| StartError(format!("no se pudo abrir la base: {error}")))?;
	crate::services::sembrar(&db, &config.siembra)
		.await
		.map_err(|error| StartError(format!("datos iniciales: {error}")))?;
	if config.siembra.admin_clave.is_none() {
		tracing::warn!("ADMIN_CLAVE not set: there is no admin account for the /v1 admin routes");
	}

	let aws = if config.pey.live || config.events_table.is_some() {
		Some(aws_config::load_from_env().await)
	} else {
		None
	};

	let (pey, webhook_secret): (Arc<dyn PeyClient>, Option<String>) = if config.pey.live {
		let aws = aws.as_ref().ok_or_else(|| StartError("aws config missing".into()))?;
		let secret_id = config
			.pey
			.secret_id
			.as_deref()
			.ok_or_else(|| StartError("PEY_MODE=live needs PEY_SECRET_ID".into()))?;
		let secret = aws_sdk_secretsmanager::Client::new(aws)
			.get_secret_value()
			.secret_id(secret_id)
			.send()
			.await
			.map_err(|error| StartError(format!("reading the Pey secret: {error}")))?
			.secret_string
			.ok_or_else(|| StartError("the Pey secret has no text value".into()))?;
		let credentials = PeyCredentials::parse(&secret).map_err(|error| StartError(error.to_string()))?;
		let webhook_secret = credentials.webhook_secret.clone();
		let live = LivePey::new(config.pey.base_url.clone(), credentials).map_err(|error| StartError(error.to_string()))?;
		(Arc::new(live), webhook_secret)
	} else {
		let clock = Arc::new(chrono::Utc::now);
		(
			Arc::new(SimulatedPey::new(config.pey.settle_after, clock)),
			config.pey.webhook_secret.clone(),
		)
	};

	let events: Arc<dyn EventStore> = match (&config.events_table, &aws) {
		(Some(table), Some(aws)) => {
			let mut builder = aws_sdk_dynamodb::config::Builder::from(aws);
			if let Some(endpoint) = &config.dynamodb_endpoint {
				builder = builder.endpoint_url(endpoint);
			}
			let client = aws_sdk_dynamodb::Client::from_conf(builder.build());
			Arc::new(DynamoEvents::new(client, table.clone()))
		}
		_ => {
			tracing::warn!("PEY_EVENTS_TABLE not set: Pey charges are kept in memory");
			Arc::new(MemoryEvents::default())
		}
	};

	Ok(Shared {
		db,
		api: AppState {
			payments: PaymentService {
				pey,
				events,
				period: config.current_period.clone(),
				amount_override: config.pey.amount_override,
			},
			capacity: config.workshop_capacity,
			webhook_secret,
		},
	})
}

/// The app runs on phones and the web demo, from any origin.
pub fn cors() -> Cors {
	Cors::default()
		.allow_any_origin()
		.allow_any_method()
		.allow_any_header()
		.max_age(3600)
}
