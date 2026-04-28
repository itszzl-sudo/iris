//! Iris CSS Object Model (CSSOM) Implementation
//!
//! This crate provides a complete CSSOM implementation including:
//! - CSS parsing and selector matching
//! - CSS Modules support
//! - Web-compatible CSSOM APIs (CSSStyleSheet, CSSRule, CSSStyleDeclaration, etc.)
//! - Bridge layer for iris-layout integration
//!
//! # Architecture
//!
//! ```text
//! CSS Text → css.rs (Parser) → CSSRule → cssrule.rs (CSSOM Wrapper)
//!                                      ↓
//!                              stylesheet.rs (CSSStyleSheet)
//!                                      ↓
//!                              bridge.rs (Integration with iris-layout)
//! ```
//!
//! # Example
//!
//! ```rust
//! use iris_cssom::{CSSStyleSheet, CSSOMManager, CSSStyleDeclaration};
//!
//! // Create a stylesheet
//! let mut sheet = CSSStyleSheet::new();
//! sheet.insert_rule(".container { color: red; }", 0).unwrap();
//!
//! // Use CSSOMManager for multiple stylesheets
//! let mut manager = CSSOMManager::new();
//! manager.add_stylesheet_from_css("main", ".class { padding: 20px; }");
//!
//! // Use CSSStyleDeclaration for inline styles
//! let mut style = CSSStyleDeclaration::new();
//! style.set_property("font-size", "16px", "");
//! ```

#![warn(missing_docs)]

// Core CSS modules
pub mod css;
pub mod css_modules;

// CSSOM API modules
pub mod cssom;        // CSSStyleDeclaration
pub mod cssrule;      // CSSRule, CSSStyleRule, CSSMediaRule
pub mod cssrulelist;  // CSSRuleList
pub mod stylesheet;   // CSSStyleSheet
pub mod bridge;       // CSSOMManager (bridge to iris-layout)

// Re-export core types for convenience
pub use css::{Selector, SelectorType, Stylesheet, CSSRule, parse_stylesheet};
pub use css_modules::{scope_class_name, transform_css, generate_short_hash};

// Re-export CSSOM API types
pub use cssom::CSSStyleDeclaration;
pub use cssrule::{CSSRuleOM, CSSRuleType, CSSStyleRule, CSSMediaRule, CSSRuleTrait};
pub use cssrulelist::CSSRuleList;
pub use stylesheet::CSSStyleSheet;
pub use bridge::CSSOMManager;
