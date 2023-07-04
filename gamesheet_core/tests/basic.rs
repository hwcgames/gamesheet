use gamesheet_core::{GameSheet, Sheet};

#[test]
fn read_from_sheet() {
	// Parse the sheet
	let sheet = include_str!("./basic.gamesheet");
	let sheet = Sheet::parse(sheet).expect("parse example sheet");

	// Take out a read-lock on the sheet
	let sheet = sheet.read().unwrap();

	// Print the sheet's contents
	println!("{sheet:#?}");

	// Test the contents of the sheet from the start
	let constant = sheet.eval("constant").expect("get constant");
	let function = sheet.eval("function").expect("get function example");
	let prelude_call = sheet.eval("prelude").expect("get prelude example");
	assert_eq!(constant.as_float().unwrap(), 7.0);
	assert_eq!(function.as_float().unwrap(), 14.0);
	assert_eq!(prelude_call.as_float().unwrap(), 49.0);

	// Write new contents for the constant field
	sheet
		.insert_entry("constant", "8.0".to_string())
		.expect("new constant");

	// Test the new contents of the sheet
	let constant = sheet.eval("constant").expect("get constant");
	let function = sheet.eval("function").expect("get function example");
	let prelude_call = sheet.eval("prelude").expect("get prelude example");
	assert_eq!(constant.as_float().unwrap(), 8.0);
	assert_eq!(function.as_float().unwrap(), 16.0);
	assert_eq!(prelude_call.as_float().unwrap(), 56.0);
}
