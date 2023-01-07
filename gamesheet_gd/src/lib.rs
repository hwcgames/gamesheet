use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use gamesheet_core::{Dynamic, Sheet};
use gdnative::prelude::*;

lazy_static::lazy_static! {
	static ref SHEETS: RwLock<HashMap<usize, Arc<RwLock<Sheet>>>> = RwLock::new(HashMap::new());
}

#[derive(NativeClass, ToVariant, FromVariant)]
#[inherit(Object)]
pub struct GameSheet(Option<usize>);

#[methods]
impl GameSheet {
	fn new(_: &Object) -> Self {
		Self(None)
	}

	#[method]
	pub fn init(&mut self, content: String) {
		let sheet = Sheet::parse(&content);
		let mut sheets = SHEETS.write().unwrap();
		if let Ok(sheet) = sheet {
			let taken: Vec<_> = sheets.keys().copied().collect();
			let next = (0..)
				.find(|v| !taken.contains(v))
				.expect("I think that's a few too many sheets pal");
			sheets.insert(next, sheet);
			self.0 = Some(next);
		} else if let Err(e) = sheet {
			godot_error!("{}", e)
		}
	}

	#[method]
	pub fn ok(&self) -> bool {
		self.0.is_some()
	}

	fn get_sheet(&self) -> Option<Arc<RwLock<Sheet>>> {
		if let Some(id) = self.0 {
			if let Some(sheet) = SHEETS.read().unwrap().get(&id) {
				let sheet = sheet.clone();
				Some(sheet)
			} else {
				None
			}
		} else {
			None
		}
	}

	#[method]
	pub fn insert(&self, name: String, script: String) -> bool {
		if let Some(sheet) = self.get_sheet() {
			let sheet = sheet.read().unwrap();
			sheet.insert_entry(&name, script).is_ok()
		} else {
			false
		}
	}

	#[method]
	pub fn insert_prelude(&self, script: String) -> bool {
		if let Some(sheet) = self.get_sheet() {
			let mut sheet = sheet.write().unwrap();
			sheet.insert_prelude(script).is_ok()
		} else {
			false
		}
	}

	#[method]
	pub fn get(&self, name: String) -> Option<Variant> {
		let sheet = self.get_sheet()?;
		let sheet = sheet.read().ok()?;
		let value = match sheet.eval(&name) {
			Ok(v) => Some(v),
			Err(e) => {
				godot_error!("{}", e);
				None
			}
		}?;
		Some(variant_from_dynamic(&value))
	}
}

impl Drop for GameSheet {
	fn drop(&mut self) {
		if let Some(id) = self.0 {
			SHEETS.write().unwrap().remove(&id);
		}
	}
}

fn variant_from_dynamic(value: &Dynamic) -> Variant {
	if let Ok(float) = value.as_float() {
		Variant::new(float)
	} else if let Ok(int) = value.as_int() {
		Variant::new(int)
	} else if let Ok(bool) = value.as_bool() {
		Variant::new(bool)
	} else if let Ok(array) = value.clone().into_array() {
		let array: Vec<Variant> = array.iter().map(variant_from_dynamic).collect();
		Variant::new(array)
	} else {
		Variant::new(value.to_string())
	}
}

fn init(handle: InitHandle) {
	handle.add_class::<GameSheet>();
}

godot_init!(init);
