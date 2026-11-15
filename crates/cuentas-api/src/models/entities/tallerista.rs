use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "talleristas")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i32,
	pub nombre: String,
	pub oficio: String,
	pub pey_api_key_id: String,
	pub usuario: Option<String>,
	pub telefono: Option<String>,
	#[serde(skip_serializing)]
	pub clave_hash: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
