//! Settles CoDi charges whose Pey webhook never arrived: a background sweep asks Pey about
//! pending charges, often while they are fresh and less often later.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};

use super::{PaymentError, PaymentService};
use crate::models::entities::pago;

/// Asked on every sweep while this young.
const FRESH: Duration = Duration::minutes(30);
/// After that, asked at most this often.
const SLOW_EVERY: Duration = Duration::minutes(15);
/// Our QR codes have no expiration, so they can be paid late; we stop asking after this.
const QR_GIVE_UP: Duration = Duration::days(7);
/// A CoDi push expires after 6 minutes (Pey's default); this leaves room for a late bank.
const PUSH_GIVE_UP: Duration = Duration::minutes(30);

/// Whether a pending charge should be asked about now. `age` counts from its creation and
/// `since_last` from the last time this process asked (`None`: never).
pub fn is_due(method: &str, age: Duration, since_last: Option<Duration>) -> bool {
	let give_up = if method == "helper" { PUSH_GIVE_UP } else { QR_GIVE_UP };
	if age > give_up {
		return false;
	}
	age <= FRESH || since_last.is_none_or(|since| since >= SLOW_EVERY)
}

/// Result of one sweep.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Sweep {
	/// Charges Pey was asked about.
	pub checked: usize,
	/// `pagos.id` of the charges this sweep marked paid.
	pub settled: Vec<i32>,
}

/// Remembers between sweeps how each pending charge started and when Pey was last asked.
#[derive(Default)]
pub struct Reconciler {
	/// `None`: the row was not started by the app (Amanda's admin pages), so Pey never knew it.
	started: HashMap<i32, Option<(String, DateTime<Utc>)>>,
	asked: HashMap<i32, DateTime<Utc>>,
}

impl Reconciler {
	pub async fn sweep(
		&mut self,
		service: &PaymentService,
		conn: &DatabaseConnection,
		now: DateTime<Utc>,
	) -> Result<Sweep, PaymentError> {
		let pending = pago::Entity::find()
			.filter(pago::Column::Estado.eq("pendiente"))
			.filter(
				Condition::any()
					.add(pago::Column::CodiQrId.is_not_null())
					.add(pago::Column::TransactionId.is_not_null()),
			)
			.all(conn)
			.await?;
		let ids: HashSet<i32> = pending.iter().map(|row| row.id).collect();
		self.started.retain(|id, _| ids.contains(id));
		self.asked.retain(|id, _| ids.contains(id));

		let mut sweep = Sweep::default();
		for row in pending {
			let Some((method, created_at)) = self.started(service, row.id).await else {
				continue;
			};
			let since_last = self.asked.get(&row.id).map(|at| now - *at);
			if !is_due(&method, now - created_at, since_last) {
				continue;
			}
			self.asked.insert(row.id, now);
			sweep.checked += 1;
			let pago_id = row.id;
			match service.settle_with_pey(conn, row, Some(created_at)).await {
				Ok(Some(_)) => sweep.settled.push(pago_id),
				Ok(None) => {}
				Err(PaymentError::Pey(error)) => tracing::warn!(pago_id, %error, "could not ask Pey about a pending payment"),
				Err(error) => return Err(error),
			}
		}
		Ok(sweep)
	}

	/// How the charge started, read once from the events store.
	async fn started(&mut self, service: &PaymentService, pago_id: i32) -> Option<(String, DateTime<Utc>)> {
		if let Some(known) = self.started.get(&pago_id) {
			return known.clone();
		}
		let record = match service.events.charge(pago_id).await {
			Ok(record) => record,
			Err(error) => {
				tracing::warn!(pago_id, %error, "could not read how a payment started");
				return None;
			}
		};
		let started = record.and_then(|record| Some((record.method.clone(), record.created_at()?)));
		self.started.insert(pago_id, started.clone());
		started
	}
}

/// Sweeps every `every` for as long as the server runs.
pub async fn reconcile_forever(service: &PaymentService, conn: &DatabaseConnection, every: std::time::Duration) {
	let mut reconciler = Reconciler::default();
	let mut ticks = tokio::time::interval(every);
	ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	loop {
		ticks.tick().await;
		match reconciler.sweep(service, conn, Utc::now()).await {
			Ok(sweep) if !sweep.settled.is_empty() => {
				tracing::info!(settled = ?sweep.settled, "settled payments that Pey reported paid without a webhook")
			}
			Ok(_) => {}
			Err(error) => tracing::warn!(?error, "the pending payments sweep failed"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fresh_charges_are_asked_every_sweep() {
		assert!(is_due("qr", Duration::minutes(2), Some(Duration::seconds(60))));
		assert!(is_due("helper", Duration::minutes(29), Some(Duration::seconds(60))));
	}

	#[test]
	fn older_qr_charges_are_asked_every_quarter_hour_for_a_week() {
		let age = Duration::hours(5);
		assert!(is_due("qr", age, None));
		assert!(!is_due("qr", age, Some(Duration::minutes(14))));
		assert!(is_due("qr", age, Some(Duration::minutes(15))));
		assert!(is_due("qr", Duration::days(7), Some(Duration::hours(1))));
		assert!(!is_due("qr", Duration::days(7) + Duration::minutes(1), None));
	}

	#[test]
	fn an_expired_push_is_not_asked_again() {
		assert!(!is_due("helper", Duration::minutes(31), None));
	}
}
