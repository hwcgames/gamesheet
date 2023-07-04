use std::{
	collections::HashSet,
	hash::Hash,
	path::PathBuf,
	str::FromStr,
	sync::{Arc, RwLock},
};

use clap::Parser;
use eframe::{
	egui::{CentralPanel, SidePanel, TopBottomPanel, Ui},
	epaint::ahash::HashMap,
	NativeOptions,
};
use gamesheet_core::{Dynamic, GameSheet, Sheet};

#[derive(Parser)]
pub struct Args {
	/// The list of gamesheet files to use, in ascending order of importance.
	pub files: Vec<PathBuf>,
}

fn main() {
	let args = Args::parse();

	let mut layers: Vec<(_, Arc<RwLock<Sheet>>)> = args
		.files
		.iter()
		.map(|path| {
			(
				path.clone(),
				Sheet::parse(&std::fs::read_to_string(path).expect("Failed to read file"))
					.expect("File is invalid"),
			)
		})
		.collect();

	let app = dbg!(App {
		layers,
		..Default::default()
	});

	eframe::run_native(
		"Gamesheet Editor",
		NativeOptions::default(),
		Box::new(|_| Box::new(app)),
	)
	.expect("App run failed");
}

#[derive(Default, Debug)]
pub struct App {
	pub layers: Vec<(PathBuf, Arc<RwLock<Sheet>>)>,
	pub filter: String,
	pub reverse_sort: bool,
	pub edits: HashMap<(PathBuf, String), String>,
}

impl App {
	pub fn sheets(&self) -> Vec<Arc<RwLock<Sheet>>> {
		self.layers.iter().map(|(_, sheet)| sheet.clone()).collect()
	}
}

impl eframe::App for App {
	#[allow(clippy::too_many_lines)]
	fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
		TopBottomPanel::top("MenuBar").show(ctx, |ui| {
			if ui.button("Save").clicked() {
				for (path, sheet) in self.layers.iter() {
					std::fs::write(
						path,
						serde_yaml::to_string(&*sheet.read().unwrap()).unwrap(),
					)
					.expect("Couldn't write");
				}
			}
		});
		TopBottomPanel::top("SettingsBar").show(ctx, |ui| {
			ui.label("Search");
			ui.text_edit_singleline(&mut self.filter);
			ui.checkbox(&mut self.reverse_sort, "Reverse Sort");
		});

		let mut matching: HashSet<(i32, String)> = self
			.layers
			.iter()
			.flat_map(|(path, sheet)| {
				sheet
					.read()
					.unwrap()
					.entries()
					.iter()
					.map(|entry| (0, entry.clone()))
					.collect::<Vec<_>>()
			})
			.filter(|(_, name)| {
				if self.filter.is_empty() {
					true
				} else {
					name.contains(&self.filter)
				}
			})
			.collect();

		let mut finished = false;
		'outer: while !finished {
			finished = true;
			// Find dependencies that we don't have
			for (level, name) in matching.clone() {
				if level < -2 {
					break 'outer;
				}
				for sheet in self.sheets() {
					for dependency in sheet.read().unwrap().dependencies(&name) {
						if !matching.contains(&(level - 1, dependency.clone())) {
							matching.insert((level - 1, dependency));
							finished = false;
						}
					}
				}
			}
			// Find dependents that we don't have
			for (level, name) in matching.clone() {
				if level > 2 {
					break 'outer;
				}
				for sheet in self.sheets() {
					for dependent in sheet.read().unwrap().dependents(&name) {
						if !matching.contains(&(level + 1, dependent.clone())) {
							matching.insert((level + 1, dependent));
							finished = false;
						}
					}
				}
			}
		}

		let matching: Vec<_> = matching
			.into_iter()
			.filter(|(layer, name)| {
				if self.filter.is_empty() || *layer != 0 {
					true
				} else {
					name.contains(&self.filter)
				}
			})
			.collect();

		let layers: HashSet<_> = matching.iter().map(|(layer, _)| layer).copied().collect();
		let mut layers: Vec<_> = layers.into_iter().collect();
		layers.sort_unstable();

		for layer in layers {
			let mut in_layer: Vec<_> = matching
				.iter()
				.filter(|(l, _)| *l == layer)
				.map(|(_, name)| name.clone())
				.collect();
			in_layer.sort();
			if self.reverse_sort {
				in_layer.reverse();
			}

			let inside = |ui: &mut Ui| {
				ui.label(format!("Layer {layer}"));
				for name in in_layer {
					ui.separator();
					let mut found = false;
					for (path, sheet) in &self.layers {
						if !sheet.read().unwrap().entries().iter().any(|n| n == &name) {
							continue;
						}
						ui.label(format!("{name} (from {}): ", path.display()));
						ui.label(match sheet.read().unwrap().eval(&name) {
							Ok(value) => value.to_string(),
							Err(e) => e.to_string(),
						});
						found = true;

						let source = self
							.edits
							.entry((path.clone(), name.clone()))
							.or_insert_with(|| sheet.read().unwrap().get_source(&name).unwrap());
						ui.text_edit_multiline(source);
						if *source != sheet.read().unwrap().get_source(&name).unwrap() {
							if let Err(e) =
								sheet.write().unwrap().insert_entry(&name, source.clone())
							{
								ui.label(format!("error: {e}"));
							}
						}
					}
					if !found {
						ui.label(format!("{name} not found"));
					}
				}
			};
			match layer {
				..=-1 => SidePanel::left(layer.to_string()).show(ctx, inside),
				1.. => SidePanel::right(layer.to_string()).show(ctx, inside),
				_ => CentralPanel::default().show(ctx, inside),
			};
		}
	}
}
