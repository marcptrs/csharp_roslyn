use zed_extension_api as zed;

mod csharp;
mod debugger;
mod nuget;

pub use csharp::CsharpRoslynExtension;

zed::register_extension!(CsharpRoslynExtension);
