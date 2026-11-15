use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "dispositivos_nfc")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i32,
	pub tallerista_id: i32,
	pub cara: String,
	pub chip_uid: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
