//! SSRF guard for webhook URLs (D-42): a standalone, table-tested function
//! applied at write time (`SsrfGuard::check_url`) AND send time
//! (`SsrfGuard::check_addrs`).
//!
//! Non-`http(s)` schemes are always rejected. Loopback, link-local
//! (`169.254.0.0/16`, `fe80::/10`), RFC1918, unique-local (IPv6 ULA) and
//! unspecified addresses are rejected unless `allow_private` is set. The
//! cloud metadata address `169.254.169.254` is ALWAYS rejected, even with
//! `allow_private` -- it is never a legitimate webhook receiver, and
//! `allow_private` exists for internal-network testing, not for opting a
//! deployment into SSRF against its own cloud metadata endpoint.
//!
//! # DNS-rebinding limitation (known, documented -- D-42)
//!
//! [`SsrfGuard::check_url`] resolves a hostname and classifies the returned
//! addresses; [`SsrfGuard::check_addrs`] repeats the same classification at
//! send time against the addresses the HTTP client actually connects to.
//! Neither step *pins* the resolved address between the write-time check
//! and the eventual send-time connection: a hostname that answers a public
//! address at write time and a rebound private/metadata address at send
//! time (classic DNS rebinding) is caught only if the send-time check runs
//! against the SAME addresses the connection will use. This crate does not
//! implement resolve-then-connect address pinning (e.g. a custom `reqwest`
//! resolver hook) -- that is a known, deliberate limitation, not an
//! oversight, and is out of this plan's scope.

use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};
use std::pin::Pin;
use std::sync::Arc;

use thiserror::Error;
use url::Url;

/// A boxed, `Send`-able future returned by an [`SsrfGuard`]'s injectable
/// resolver closure.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// The cloud metadata address (AWS/GCP/Azure IMDS) -- always rejected,
/// regardless of `allow_private` (D-42).
const METADATA_ADDR: Ipv4Addr = Ipv4Addr::new(169, 254, 169, 254);

/// Why an [`SsrfGuard`] check rejected a URL or resolved address.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum SsrfRejection {
    /// The URL's scheme is not `http` or `https`.
    #[error("scheme not http(s): {scheme}")]
    SchemeNotHttp {
        /// The rejected scheme.
        scheme: String,
    },
    /// A loopback address (`127.0.0.0/8`, `::1`).
    #[error("loopback address rejected")]
    Loopback,
    /// A link-local address (`169.254.0.0/16`, `fe80::/10`).
    #[error("link-local address rejected")]
    LinkLocal,
    /// An RFC1918 private address.
    #[error("private (RFC1918) address rejected")]
    Private,
    /// An IPv6 unique-local (ULA) address.
    #[error("unique-local (IPv6 ULA) address rejected")]
    UniqueLocal,
    /// The unspecified address (`0.0.0.0`, `::`).
    #[error("unspecified address rejected")]
    Unspecified,
    /// The cloud metadata address `169.254.169.254` -- always rejected,
    /// even with `allow_private` set.
    #[error("cloud metadata address (169.254.169.254) rejected")]
    Metadata,
    /// The hostname did not resolve to any address.
    #[error("host {host} did not resolve to any address")]
    Unresolvable {
        /// The unresolvable hostname.
        host: String,
    },
    /// The URL itself was malformed.
    #[error("invalid URL: {message}")]
    InvalidUrl {
        /// Description of the parse failure.
        message: String,
    },
}

/// The write-time-AND-send-time SSRF guard (D-42).
///
/// Cloning is cheap: the injectable resolver is held behind an `Arc`.
#[derive(Clone)]
pub struct SsrfGuard {
    allow_private: bool,
    resolver: Arc<dyn Fn(&str) -> BoxFuture<std::io::Result<Vec<IpAddr>>> + Send + Sync>,
}

impl std::fmt::Debug for SsrfGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SsrfGuard")
            .field("allow_private", &self.allow_private)
            .finish_non_exhaustive()
    }
}

fn default_resolver(host: &str) -> BoxFuture<std::io::Result<Vec<IpAddr>>> {
    let host = host.to_string();
    Box::pin(async move {
        let addrs: Vec<IpAddr> = tokio::net::lookup_host((host.as_str(), 0u16))
            .await?
            .map(|socket_addr| socket_addr.ip())
            .collect();
        Ok(addrs)
    })
}

impl SsrfGuard {
    /// Construct a guard with the real `tokio::net::lookup_host`-backed
    /// resolver.
    pub fn new(allow_private: bool) -> Self {
        Self {
            allow_private,
            resolver: Arc::new(default_resolver),
        }
    }

    /// Override the resolver -- tests inject a deterministic, no-network
    /// closure so `check_url`'s hostname-resolution path is exercised
    /// without depending on real DNS.
    pub fn with_resolver(
        mut self,
        resolver: Arc<dyn Fn(&str) -> BoxFuture<std::io::Result<Vec<IpAddr>>> + Send + Sync>,
    ) -> Self {
        self.resolver = resolver;
        self
    }

    /// Whether this guard was constructed with `allow_private: true`.
    pub fn allow_private(&self) -> bool {
        self.allow_private
    }

    /// Write-time check (D-42): validate the scheme, then classify the
    /// host -- an IP literal directly, or a hostname via the resolver, with
    /// EVERY returned address required to pass.
    ///
    /// # Errors
    ///
    /// Returns an [`SsrfRejection`] naming the specific violation.
    pub async fn check_url(&self, url: &str) -> Result<Url, SsrfRejection> {
        let parsed = Url::parse(url).map_err(|e| SsrfRejection::InvalidUrl {
            message: e.to_string(),
        })?;

        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(SsrfRejection::SchemeNotHttp {
                scheme: scheme.to_string(),
            });
        }

        let host = parsed.host_str().ok_or_else(|| SsrfRejection::InvalidUrl {
            message: "URL has no host".to_string(),
        })?;

        if let Some(ip) = literal_ip(host) {
            self.check_addrs(&[ip])?;
        } else {
            let addrs = (self.resolver)(host)
                .await
                .map_err(|_| SsrfRejection::Unresolvable {
                    host: host.to_string(),
                })?;
            if addrs.is_empty() {
                return Err(SsrfRejection::Unresolvable {
                    host: host.to_string(),
                });
            }
            self.check_addrs(&addrs)?;
        }

        Ok(parsed)
    }

    /// Send-time check (D-42): classify already-resolved addresses (e.g.
    /// the addresses the HTTP client is about to connect to).
    ///
    /// # Errors
    ///
    /// Returns an [`SsrfRejection`] naming the specific violation on the
    /// FIRST address that fails.
    pub fn check_addrs(&self, addrs: &[IpAddr]) -> Result<(), SsrfRejection> {
        for addr in addrs {
            self.check_addr(*addr)?;
        }
        Ok(())
    }

    fn check_addr(&self, addr: IpAddr) -> Result<(), SsrfRejection> {
        let effective = to_ipv4_mapped(addr).unwrap_or(addr);

        // The metadata address is checked BEFORE the allow_private
        // short-circuit -- it is always rejected (D-42).
        if effective == IpAddr::V4(METADATA_ADDR) {
            return Err(SsrfRejection::Metadata);
        }

        if self.allow_private {
            return Ok(());
        }

        match effective {
            IpAddr::V4(v4) => {
                if v4.is_loopback() {
                    return Err(SsrfRejection::Loopback);
                }
                if v4.is_link_local() {
                    return Err(SsrfRejection::LinkLocal);
                }
                if v4.is_private() {
                    return Err(SsrfRejection::Private);
                }
                if v4.is_unspecified() {
                    return Err(SsrfRejection::Unspecified);
                }
            }
            IpAddr::V6(v6) => {
                if v6.is_loopback() {
                    return Err(SsrfRejection::Loopback);
                }
                if v6.is_unicast_link_local() {
                    return Err(SsrfRejection::LinkLocal);
                }
                if v6.is_unique_local() {
                    return Err(SsrfRejection::UniqueLocal);
                }
                if v6.is_unspecified() {
                    return Err(SsrfRejection::Unspecified);
                }
            }
        }
        Ok(())
    }
}

/// Parse `host` as an IP literal directly, or -- if every character is an
/// ASCII digit -- as a decimal-encoded IPv4 address (`http://2130706433` ==
/// `http://127.0.0.1`). `url::Url` already normalizes most decimal-IPv4
/// hosts into dotted-quad form during parsing, but this fallback covers any
/// host string that reaches here unnormalized.
fn literal_ip(host: &str) -> Option<IpAddr> {
    // `Url::host_str()` renders an IPv6 host bracketed (`"[::1]"`), matching
    // URL syntax -- strip the brackets before handing the address to
    // `IpAddr::parse`, which expects the bare form.
    let unbracketed = host.strip_prefix('[').and_then(|s| s.strip_suffix(']'));
    if let Ok(ip) = unbracketed.unwrap_or(host).parse::<IpAddr>() {
        return Some(ip);
    }
    if !host.is_empty() && host.chars().all(|c| c.is_ascii_digit()) {
        let value: u32 = host.parse().ok()?;
        return Some(IpAddr::V4(Ipv4Addr::from(value)));
    }
    None
}

/// If `addr` is an IPv4-mapped IPv6 address (`::ffff:a.b.c.d`), return the
/// mapped IPv4 address so it is classified by its REAL address rather than
/// slipping past the IPv6 branch's own checks.
fn to_ipv4_mapped(addr: IpAddr) -> Option<IpAddr> {
    match addr {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4),
        IpAddr::V4(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_resolver(
        answer: Vec<IpAddr>,
    ) -> Arc<dyn Fn(&str) -> BoxFuture<std::io::Result<Vec<IpAddr>>> + Send + Sync> {
        Arc::new(move |_host: &str| {
            let answer = answer.clone();
            Box::pin(async move { Ok(answer) })
        })
    }

    async fn assert_rejected(url: &str, allow_private: bool) {
        let guard = SsrfGuard::new(allow_private)
            .with_resolver(stub_resolver(vec!["127.0.0.1".parse().unwrap()]));
        let result = guard.check_url(url).await;
        assert!(
            result.is_err(),
            "expected {url} (allow_private={allow_private}) to be rejected, got {result:?}"
        );
    }

    async fn assert_accepted(url: &str, allow_private: bool, resolved: Vec<IpAddr>) {
        let guard = SsrfGuard::new(allow_private).with_resolver(stub_resolver(resolved));
        let result = guard.check_url(url).await;
        assert!(
            result.is_ok(),
            "expected {url} (allow_private={allow_private}) to be accepted, got {result:?}"
        );
    }

    #[tokio::test]
    async fn webhook_ssrf_guard() {
        // Non-http(s) schemes -- ALWAYS rejected, regardless of allow_private.
        for url in ["ftp://x", "file:///etc/passwd"] {
            assert_rejected(url, false).await;
            assert_rejected(url, true).await;
        }

        // Metadata address -- ALWAYS rejected, even with allow_private.
        for url in ["http://169.254.169.254/latest/meta-data"] {
            assert_rejected(url, false).await;
            assert_rejected(url, true).await;
        }

        // Loopback / link-local / private / unspecified IP literals --
        // rejected with allow_private == false, accepted with true.
        let literal_cases = [
            "http://127.0.0.1",
            "http://[::1]",
            "http://10.0.0.1",
            "http://172.16.5.5",
            "http://192.168.1.1",
            "http://169.254.1.1",
            "http://[fe80::1]",
            "http://[fd00::1]",
            "http://0.0.0.0",
            "http://[::]",
            "http://2130706433",
            "http://[::ffff:127.0.0.1]",
        ];
        for url in literal_cases {
            assert_rejected(url, false).await;
            assert_accepted(url, true, vec![]).await;
        }

        // localhost -- resolved via the injectable resolver stubbed to
        // 127.0.0.1 (as documented: real DNS is never exercised in tests).
        assert_rejected("http://localhost", false).await;
        assert_accepted("http://localhost", true, vec!["127.0.0.1".parse().unwrap()]).await;

        // Accepted: a public hostname (resolved via a stubbed public
        // address) and a public IP literal.
        assert_accepted(
            "https://hooks.example.com/x",
            false,
            vec!["8.8.8.8".parse().unwrap()],
        )
        .await;
        assert_accepted("http://8.8.8.8", false, vec![]).await;
    }

    #[test]
    fn webhook_ssrf_guard_metadata_constant_matches_the_documented_address() {
        assert_eq!(METADATA_ADDR.to_string(), "169.254.169.254");
    }

    #[tokio::test]
    async fn check_addrs_is_the_send_time_half_of_the_same_guard() {
        let guard = SsrfGuard::new(false);
        let err = guard
            .check_addrs(&["127.0.0.1".parse().unwrap()])
            .unwrap_err();
        assert_eq!(err, SsrfRejection::Loopback);

        let err = guard.check_addrs(&[IpAddr::V4(METADATA_ADDR)]).unwrap_err();
        assert_eq!(err, SsrfRejection::Metadata);

        assert!(guard.check_addrs(&["8.8.8.8".parse().unwrap()]).is_ok());
    }

    #[tokio::test]
    async fn unresolvable_host_is_rejected() {
        let guard = SsrfGuard::new(false).with_resolver(stub_resolver(vec![]));
        let err = guard
            .check_url("https://nowhere.invalid")
            .await
            .unwrap_err();
        assert!(matches!(err, SsrfRejection::Unresolvable { .. }));
    }
}
