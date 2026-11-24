#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

mod entity;
mod generated;
mod graph;

pub use entity::{Entity, IntoProperty, Property as PropertyInput, Reference, SchemaType};
pub use generated::*;
pub use graph::{Error as CollectionError, Graph, MultiTypedEntity};

/// Default JSON-LD context.
pub const CONTEXT: &str = "https://schema.org";

/// Render JSON-LD as JSON or a script safe to embed in HTML.
pub trait JsonLd {
    /// Return the JSON-LD document.
    fn to_value(&self) -> serde_json::Value;

    /// Return compact JSON.
    fn to_json(&self) -> String {
        self.to_value().to_string()
    }

    /// Return the Content Security Policy nonce, if configured.
    fn nonce(&self) -> String;

    /// Return a JSON-LD script. HTML delimiters in JSON and nonce attributes are escaped.
    fn to_script(&self) -> String {
        let nonce = self.nonce();
        let attr = if nonce.is_empty() {
            String::new()
        } else {
            format!(
                " nonce=\"{}\"",
                nonce
                    .replace('&', "&amp;")
                    .replace('"', "&quot;")
                    .replace('\'', "&#039;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            )
        };
        let json = self
            .to_json()
            .replace('<', "\\u003C")
            .replace('>', "\\u003E");
        format!("<script type=\"application/ld+json\"{attr}>{json}</script>")
    }
}

#[cfg(doctest)]
mod guide {
    include!(concat!(env!("OUT_DIR"), "/documentation.rs"));
}
