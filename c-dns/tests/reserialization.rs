use color_eyre::eyre::Result;
use pretty_assertions::assert_eq;
use serde_cbor::Value;

/// Test that parsing and re-serializing the C-DNS file will not loose any fields.
///
/// Some fields have negative indices, for custom extensions, but they need to be captured manually.
#[test]
fn reserialize_file() -> Result<()> {
    let c_dns_content = std::fs::read("./tests/data/dns.cdns")?;

    let before: Value = serde_cbor::from_slice(&c_dns_content)?;
    let c_dns_file: c_dns::serialization::File = serde_cbor::from_slice(&c_dns_content)?;
    let after_content = serde_cbor::to_vec(&c_dns_file)?;
    let after: Value = serde_cbor::from_slice(&after_content)?;

    assert_eq!(before, after);
    Ok(())
}
