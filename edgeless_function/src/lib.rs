pub mod api {
    wit_bindgen::generate!({world: "edgefunction", macro_export, export_macro_name: "export"});
}
