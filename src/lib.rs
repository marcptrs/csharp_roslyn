use zed_extension_api as zed;

mod csharp;
mod debugger;
mod logging;
mod omnisharp_download;
mod project_info;

pub use csharp::CsharpRoslynExtension;

zed::register_extension!(CsharpRoslynExtension);
