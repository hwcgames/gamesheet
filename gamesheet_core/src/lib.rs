//! GameSheet is a library that provides a simple system for storing and computing parameters for game behavior.

use std::{
	fmt::Display,
	sync::{Arc, RwLock},
};

use dashmap::DashMap;
use rhai::{ASTNode, Engine, EvalAltResult, Expr, AST};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Convenience re-export of Rhai's Dynamic, since that's what we return.
pub use rhai::Dynamic;

/// The structure that holds the entire GameSheet.
/// It also holds a Rhai execution engine for evaluating scripts.
#[derive(Serialize, Deserialize, Debug)]
pub struct Sheet {
	#[serde(skip)]
	engine: Engine,
	/// The Rhai script prelude. It can set up functions for the other systems to use.
	prelude: String,
	#[serde(skip)]
	prelude_ast: Option<AST>,
	/// A map to Rhai scripts. Their dependencies will be determined automatically, and their values will be lazily cached and evicted.
	/// Scripts must be deterministic, since they will only run once.
	entries: DashMap<String, String>,
	/// Cached script ASTs, without the prelude included.
	#[serde(skip)]
	asts: DashMap<String, AST>,
	/// Cached script dependencies.
	#[serde(skip)]
	deps: DashMap<String, Vec<String>>,
	/// Cached script results.
	#[serde(skip)]
	cache: DashMap<String, Dynamic>,
}

#[derive(Debug, Error)]
pub enum SheetError {
	BadYaml(#[from] serde_yaml::Error),
	BadScript(#[from] rhai::ParseError),
	CyclicDependency(String),
	BadDependency(String),
	UnexpectedResult(Dynamic),
	EvalFailure(#[from] Box<EvalAltResult>),
}

impl Display for SheetError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&format!("{self:?}"))
	}
}

impl Sheet {
	pub fn parse(s: &str) -> Result<Arc<RwLock<Self>>, SheetError> {
		// Parse the sheet from yaml
		let sheet_: Arc<RwLock<Sheet>> = Arc::new(RwLock::new(serde_yaml::from_str(s)?));
		{
			let mut sheet = sheet_.write().unwrap();
			let sheet_ = sheet_.clone();
			sheet.engine.register_fn("g", move |name: &str| {
				match sheet_.read().unwrap().eval(name) {
					Err(e) => {
						eprintln!("Inner evaluation failed with {e}");
						Dynamic::UNIT
					}
					Ok(f) => f,
				}
			});
			// Compile all of the scripts
			sheet.build_prelude()?;
			for p in sheet.entries.iter().map(|v| v.pair().0.to_string()) {
				sheet.build_entry(&p)?;
			}
			// Confirm that every script's dependencies actually exists
			for dep in sheet.deps.iter().flat_map(|v| v.pair().1.clone()) {
				if !sheet.entries.contains_key(&dep) {
					return Err(SheetError::BadDependency(dep));
				}
			}
		}
		Ok(sheet_)
	}
	pub fn eval(&self, name: &str) -> Result<Dynamic, SheetError> {
		if let Some(cache) = self.cache.get(name) {
			Ok(cache.pair().1.clone())
		} else if let Some(ast) = self.asts.get(name) {
			let ast = ast.pair().1;
			let ast = self
				.prelude_ast
				.as_ref()
				.expect("AST must exist")
				.clone()
				.merge(ast);
			let outcome: Dynamic = self.engine.eval_ast(&ast)?;
			self.cache.insert(name.to_string(), outcome.clone());
			Ok(outcome)
		} else {
			Err(SheetError::BadDependency(name.to_string()))
		}
	}
	pub fn insert_entry(&self, name: &str, script: String) -> Result<(), SheetError> {
		match self.engine.compile(script.clone()) {
			Ok(_) => {
				self.entries.insert(name.to_string(), script);
				self.build_entry(name)?;
				Ok(())
			}
			Err(e) => Err(SheetError::BadScript(e)),
		}
	}
	pub fn insert_prelude(&mut self, script: String) -> Result<(), SheetError> {
		self.prelude = script;
		self.build_prelude()?;
		Ok(())
	}
	fn build_prelude(&mut self) -> Result<(), SheetError> {
		#[cfg(debug_assertions)]
		println!("Compiling AST for prelude");
		self.prelude_ast = Some(self.engine.compile(&self.prelude)?);
		self.cache.clear();
		Ok(())
	}
	fn build_entry(&self, name: &str) -> Result<(), SheetError> {
		if let Some(value) = self.entries.get(name) {
			// Compile the script itself
			#[cfg(debug_assertions)]
			println!("Compiling AST for {name}");
			let ast = self.engine.compile(&*value)?;
			self.asts.insert(name.to_string(), ast.clone());
			// Determine this scripts's dependencies
			let mut deps = vec![];
			ast.walk(&mut |nodes| {
				for node in nodes {
					if let ASTNode::Expr(Expr::FnCall(expr, _)) = node {
						// This expression is a function call
						if expr.name == "g" {
							// This expression is getting another value
							if let Some(Expr::StringConstant(str, _)) = expr.args.get(0) {
								deps.push(str.to_string());
							}
						}
					}
				}
				true
			});
			self.deps.insert(name.to_string(), deps);
			// Invalidate result cache for this script
			self.invalidate_cache(name, &[])?;
			Ok(())
		} else {
			Ok(())
		}
	}
	fn invalidate_cache(&self, name: &str, bad_parents: &[String]) -> Result<(), SheetError> {
		self.cache.remove(name);
		let mut parents = bad_parents.to_owned();
		parents.push(name.to_string());
		for dependent in self
			.deps
			.iter()
			.filter(|p| p.pair().1.contains(&name.to_string()))
			.map(|p| p.pair().0.to_string())
		{
			if bad_parents.contains(&dependent) {
				return Err(SheetError::CyclicDependency(dependent));
			}
			self.invalidate_cache(&dependent, &parents)?;
		}
		Ok(())
	}
}
