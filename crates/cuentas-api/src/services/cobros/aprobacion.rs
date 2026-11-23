//! Which accounts may start real CoDi charges (`cobros_reales`, see `migrations/0005_…`).

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};

pub async fn is_approved(conn: &DatabaseConnection, tallerista_id: i32) -> Result<bool, DbErr> {
	let row = conn
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT 1 FROM cobros_reales WHERE tallerista_id = $1",
			[tallerista_id.into()],
		))
		.await?;
	Ok(row.is_some())
}

/// Approves the account; `false` when it does not exist.
pub async fn approve(conn: &DatabaseConnection, tallerista_id: i32) -> Result<bool, DbErr> {
	let result = conn
		.execute_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"INSERT INTO cobros_reales (tallerista_id)
			 SELECT id FROM talleristas WHERE id = $1
			 ON CONFLICT (tallerista_id) DO NOTHING",
			[tallerista_id.into()],
		))
		.await?;
	Ok(result.rows_affected() > 0 || is_approved(conn, tallerista_id).await?)
}

/// Takes the approval away; `false` when the account was not approved.
pub async fn revoke(conn: &DatabaseConnection, tallerista_id: i32) -> Result<bool, DbErr> {
	let result = conn
		.execute_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"DELETE FROM cobros_reales WHERE tallerista_id = $1",
			[tallerista_id.into()],
		))
		.await?;
	Ok(result.rows_affected() > 0)
}

/// Approves the account whose `pey_api_key_id` is given, if it exists. Used for the demo account.
pub async fn approve_by_pey_key(conn: &DatabaseConnection, pey_api_key_id: &str) -> Result<(), DbErr> {
	conn
		.execute_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"INSERT INTO cobros_reales (tallerista_id)
			 SELECT id FROM talleristas WHERE pey_api_key_id = $1
			 ON CONFLICT (tallerista_id) DO NOTHING",
			[pey_api_key_id.into()],
		))
		.await?;
	Ok(())
}
