use zed_extension_api as zed;

mod csharp;
mod debugger;
mod roslyn_download;
mod wrapper_download;

pub use csharp::CsharpRoslynExtension;

zed::register_extension!(CsharpRoslynExtension);
