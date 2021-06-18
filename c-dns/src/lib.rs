mod iterators;
pub mod serialization;
mod utils;

/// DNS transport protocol
pub enum Transport {
    /// UDP specified in RFC 1035
    Udp = 0,
    /// TCP specified in RFC 1035
    Tcp = 1,
    /// TLS specified in RFC 7858
    Tls = 2,
    /// DTLS specified in RFC 8094
    Dtls = 3,
    /// HTTPS specified in RFC 8484
    Https = 4,
    /// Reserved Value
    Reserved = 5,
    NonStandard = 15,
}
