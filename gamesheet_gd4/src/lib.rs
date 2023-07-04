use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use gamesheet_core::{Dynamic, GameSheet as GameSheetTrait, Sheet};
use godot::prelude::{
	gdextension, godot_api, godot_error, Array, Base, ExtensionLibrary, Gd, GodotClass,
	GodotString, Object, ObjectVirtual, Variant,
};

lazy_static::lazy_static! {
	static ref SHEETS: RwLock<HashMap<usize, Arc<RwLock<Sheet>>>> = RwLock::new(HashMap::new());
}

pub struct GameSheetApi;

#[gdextension]
unsafe impl ExtensionLibrary for GameSheetApi {}

#[derive(GodotClass)]
#[class(base=Object)]
pub struct GameSheet {
	handle: Option<usize>,
	#[base]
	base: Base<Object>,
}

#[godot_api]
impl ObjectVirtual for GameSheet {
	fn init(base: Base<Object>) -> Self {
		Self { handle: None, base }
	}
}

#[godot_api]
impl GameSheet {
	#[func]
	pub fn load(&mut self, content: GodotString) {
		let content = content.to_string();
		let sheet = Sheet::parse(&content);
		let mut sheets = SHEETS.write().unwrap();
		if let Ok(sheet) = sheet {
			let taken: Vec<_> = sheets.keys().copied().collect();
			let next = (0..)
				.find(|v| !taken.contains(v))
				.expect("I think that's a few too many sheets pal");
			sheets.insert(next, sheet);
			self.handle = Some(next);
		} else if let Err(e) = sheet {
			godot_error!("{}", e)
		}
	}

	#[func]
	pub fn ok(&self) -> bool {
		self.handle.is_some()
	}

	fn get_sheet(&self) -> Option<Arc<RwLock<Sheet>>> {
		if let Some(id) = self.handle {
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

	#[func]
	pub fn insert(&self, name: GodotString, script: GodotString) -> bool {
		let name = name.to_string();
		let script = script.to_string();
		if let Some(sheet) = self.get_sheet() {
			let sheet = sheet.read().unwrap();
			sheet.insert_entry(&name, script).is_ok()
		} else {
			false
		}
	}

	#[func]
	pub fn insert_prelude(&self, script: GodotString) -> bool {
		let script = script.to_string();
		if let Some(sheet) = self.get_sheet() {
			let mut sheet = sheet.write().unwrap();
			sheet.insert_prelude(script).is_ok()
		} else {
			false
		}
	}

	/// Currently always returns String because of a bug in the gdextension bindings.
	/// May return other types in the future.
	#[func]
	pub fn get_key(&self, name: GodotString) -> GodotString {
		let out: Option<_> = (|| {
			let name = name.to_string();
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
		})();
		GodotString::from(out.unwrap_or_else(Variant::nil).to_string())
	}
}

impl Drop for GameSheet {
	fn drop(&mut self) {
		if let Some(id) = self.handle {
			SHEETS.write().unwrap().remove(&id);
		}
	}
}

fn variant_from_dynamic(value: &Dynamic) -> Variant {
	if let Ok(float) = value.as_float() {
		Variant::from(float)
	} else if let Ok(int) = value.as_int() {
		Variant::from(int)
	} else if let Ok(bool) = value.as_bool() {
		Variant::from(bool)
	} else if let Ok(array) = value.clone().into_array() {
		let array: Array<Variant> = array.iter().map(variant_from_dynamic).collect();
		Variant::from(array)
	} else {
		Variant::from(value.to_string())
	}
}
