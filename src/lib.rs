use zed_extension_api as zed;

mod csharp;
mod debugger;
mod language_servers;
mod logging;
mod project_info;

pub use csharp::CsharpRoslynExtension;

zed::register_extension!(CsharpRoslynExtension);
