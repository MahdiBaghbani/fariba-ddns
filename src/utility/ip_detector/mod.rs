//! IP Detection Module
//!
//! This module provides functionality for detecting public IP addresses using
//! multiple detection methods and consensus validation. It supports both IPv4
//! and IPv6 detection through various services and local network interfaces.
//!
//! # Features
//!
//! - Multiple IP detection services
//! - Consensus-based validation
//! - IPv4 and IPv6 support
//! - Configurable timeouts
//! - Error recovery and fallback
//!
//! # Architecture
//!
//! The module is organized into several components:
//! - Detection services implementing the `IpDetector` trait
//! - Consensus manager for validating results
//! - Error handling and recovery
//! - Configuration management
//!
//! # Example
//!
//! ```rust
//! use fariba_ddns::utility::ip_detector::{IpDetector, IpDetectorConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = IpDetectorConfig {
//!     services: vec!["ipify".to_string(), "cloudflare".to_string()],
//!     consensus_threshold: 2,
//!     timeout: std::time::Duration::from_secs(10),
//! };
//!
//! let detector = IpDetector::new(config)?;
//! let ip = detector.detect_ip().await?;
//! println!("Detected IP: {}", ip);
//! # Ok(())
//! # }
//! ```
//!
//! # Error Handling
//!
//! The module uses custom error types to handle various failure scenarios:
//! - Service timeouts
//! - Network errors
//! - Invalid responses
//! - Consensus failures
//!
//! # Configuration
//!
//! Services can be configured through the `IpDetectorConfig` struct:
//! - List of services to use
//! - Consensus threshold
//! - Request timeouts
//! - Retry settings

pub mod constants;
pub mod errors;
pub mod impls;
pub mod traits;
pub mod types;
