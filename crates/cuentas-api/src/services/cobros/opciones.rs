//! What a student can be charged in a workshop, mapped onto Amanda's `pagos.tipo` and `periodo`.

use crate::models::entities::taller;

const MONTHS: [&str; 12] = [
	"Enero",
	"Febrero",
	"Marzo",
	"Abril",
	"Mayo",
	"Junio",
	"Julio",
	"Agosto",
	"Septiembre",
	"Octubre",
	"Noviembre",
	"Diciembre",
];

/// Annual enrollment, as in Amanda's `INSCRIPCION_ANUAL_MXN`.
const ANNUAL_ID: &str = "extra-anual";
pub const CUSTOM_ID: &str = "custom";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChargeOption {
	pub id: String,
	pub label: String,
	pub detail: String,
	pub amount: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargeKind {
	/// Counts for "ya pagó" in the roster.
	Monthly,
	Annual,
	/// An amount typed by hand; stored as a `mensualidad` row in its own `periodo`, so it never
	/// counts as the month's fee.
	Custom,
}

/// A charge ready to become a `pagos` row.
#[derive(Clone, Debug)]
pub struct Charge {
	pub kind: ChargeKind,
	pub label: String,
	pub amount: i32,
	pub periodo: String,
}

impl Charge {
	pub fn tipo(&self) -> &'static str {
		match self.kind {
			ChargeKind::Annual => "inscripcion_anual",
			ChargeKind::Monthly | ChargeKind::Custom => "mensualidad",
		}
	}
}

fn month_name(period: &str) -> Option<&'static str> {
	let month: usize = period.get(5..7)?.parse().ok()?;
	MONTHS.get(month.checked_sub(1)?).copied()
}

fn year(period: &str) -> &str {
	period.get(0..4).unwrap_or(period)
}

/// The workshop's charges for the period, at most three so "Otro monto" fits as the fourth.
pub fn options_for(workshop: &taller::Model, period: &str) -> Vec<ChargeOption> {
	let month = month_name(period).unwrap_or("Mensualidad");
	vec![
		ChargeOption {
			id: format!("monthly-{period}"),
			label: month.to_string(),
			detail: "Mensualidad".to_string(),
			amount: workshop.cuota_mensual_mxn,
		},
		ChargeOption {
			id: format!("{ANNUAL_ID}-{}", year(period)),
			label: "Inscripción anual".to_string(),
			detail: "Una vez al año".to_string(),
			amount: crate::services::INSCRIPCION_ANUAL_MXN,
		},
	]
}

/// Turns an option id (or `custom` with a typed amount) into a charge.
pub fn resolve(workshop: &taller::Model, period: &str, option_id: &str, typed: f64) -> Option<Charge> {
	if option_id == CUSTOM_ID {
		if !(typed.is_finite() && (1.0..=100_000.0).contains(&typed)) {
			return None;
		}
		return Some(Charge {
			kind: ChargeKind::Custom,
			label: "Otro cobro".to_string(),
			amount: typed.round() as i32,
			periodo: format!("{period}-otro"),
		});
	}
	let option = options_for(workshop, period)
		.into_iter()
		.find(|item| item.id == option_id)?;
	let kind = if option.id.starts_with(ANNUAL_ID) {
		ChargeKind::Annual
	} else {
		ChargeKind::Monthly
	};
	Some(Charge {
		kind,
		periodo: if kind == ChargeKind::Annual {
			year(period).to_string()
		} else {
			period.to_string()
		},
		label: option.label,
		amount: option.amount,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn tejido() -> taller::Model {
		taller::Model {
			id: 7,
			tallerista_id: 3,
			nombre: "Taller de tejido".into(),
			cuota_mensual_mxn: 130,
			activo: 1,
		}
	}

	#[test]
	fn offers_the_month_and_the_annual_enrollment() {
		let options = options_for(&tejido(), "2026-12");
		assert_eq!(
			options[0],
			ChargeOption {
				id: "monthly-2026-12".into(),
				label: "Diciembre".into(),
				detail: "Mensualidad".into(),
				amount: 130
			}
		);
		assert_eq!(options[1].id, "extra-anual-2026");
		assert_eq!(options[1].amount, 125);
	}

	#[test]
	fn maps_options_to_amandas_tipo_and_periodo() {
		let monthly = resolve(&tejido(), "2026-12", "monthly-2026-12", 0.0).unwrap();
		assert_eq!((monthly.tipo(), monthly.periodo.as_str()), ("mensualidad", "2026-12"));
		let annual = resolve(&tejido(), "2026-12", "extra-anual-2026", 0.0).unwrap();
		assert_eq!((annual.tipo(), annual.periodo.as_str()), ("inscripcion_anual", "2026"));
		let custom = resolve(&tejido(), "2026-12", "custom", 75.0).unwrap();
		assert_eq!((custom.amount, custom.periodo.as_str()), (75, "2026-12-otro"));
		assert!(resolve(&tejido(), "2026-12", "custom", 0.0).is_none());
		assert!(resolve(&tejido(), "2026-12", "monthly-2027-01", 0.0).is_none());
	}
}
