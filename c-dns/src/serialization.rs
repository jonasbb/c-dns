//! Types for de-/serializing C-DNS data
//!
//! These types follow the definitions of the [C-DNS format] and are not optimized for reading the data.
//! They are intended to provide lossless deserialization and re-serialization of C-DNS data.
//! They contain references to other parts of the file (`*_index` fields) and some data (like IP addresses) can only be parsed with additional context.
//!
//! [C-DNS format]: https://tools.ietf.org/html/rfc8618

// Needed to make stuff work between stable and nightly
#![allow(renamed_and_removed_lints, clippy::unknown_clippy_lints)]
#![allow(clippy::upper_case_acronyms)]

use color_eyre::eyre::bail;
use enumset::{EnumSet, EnumSetType};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_indexed::{DeserializeIndexed, SerializeIndexed};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_tuple::{Deserialize_tuple, Serialize_tuple};
use serde_with::skip_serializing_none;
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::collections::BTreeMap;

// /////////////////////////////////////////////////////////////////////////////
// This section contains basic types common for all parts of the format
// /////////////////////////////////////////////////////////////////////////////

/// DNS Class
///
/// 16-bit type carrying class information.
///
/// List of standarized DNS classes:
/// <https://www.iana.org/assignments/dns-parameters/dns-parameters.xhtml#dns-parameters-2>
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct DnsClass(u16);

impl fmt::Debug for DnsClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("DnsClass({})", self.0))
    }
}

impl From<DnsClass> for u16 {
    fn from(value: DnsClass) -> Self {
        value.0
    }
}

impl From<u16> for DnsClass {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

/// DNS Resource Record Type
///
/// 16-bit type carrying resource record type information.
///
/// List of standarized DNS resource record types:
/// <https://www.iana.org/assignments/dns-parameters/dns-parameters.xhtml#dns-parameters-4>
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct DnsType(u16);

impl fmt::Debug for DnsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("DnsType({})", self.0))
    }
}

impl From<DnsType> for u16 {
    fn from(value: DnsType) -> Self {
        value.0
    }
}

impl From<u16> for DnsType {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

/// IPv4 or IPv6 address
///
/// Type representing an IPv4 or IPv6 address.
/// The IP address is stored as a sequence of bytes.
///
/// If client or server address prefixes are set, only the address prefix bits are stored.
/// Each string is therefore up to 4 bytes long for an IPv4 address, or up to 16 bytes long for an IPv6 address.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct IpAddr(ByteBuf);

impl fmt::Debug for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("IpAddr({:?})", self.0))
    }
}

impl IpAddr {
    pub fn as_ipv4(&self) -> color_eyre::eyre::Result<Ipv4Addr> {
        Ok(match self.0.as_slice() {
            &[] => bail!("No bytes to convert into Ipv4Addr"),
            &[a] => Ipv4Addr::new(a, 0, 0, 0),
            &[a, b] => Ipv4Addr::new(a, b, 0, 0),
            &[a, b, c] => Ipv4Addr::new(a, b, c, 0),
            &[a, b, c, d] => Ipv4Addr::new(a, b, c, d),
            bytes => bail!(
                "Too many bytes to convert into Ipv4Addr. Expected up to 4 bytes but got {}.",
                bytes.len()
            ),
        })
    }

    pub fn as_ipv6(&self) -> color_eyre::eyre::Result<Ipv6Addr> {
        use std::convert::TryFrom;

        Ok(match self.0.as_slice() {
            &[] => bail!("No bytes to convert into Ipv6Addr"),
            bytes if bytes.len() <= 16 => {
                let mut vec = bytes.to_vec();
                vec.extend(std::iter::repeat(0).take(16 - vec.len()));
                Ipv6Addr::from(<[u8; 16]>::try_from(&*vec).unwrap())
            }
            bytes => bail!(
                "Too many bytes to convert into Ipv6Addr. Expected up to 16 bytes but got {}.",
                bytes.len()
            ),
        })
    }
}

/// Holds a Name or RDATA
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct NameOrRdata(ByteBuf);

impl NameOrRdata {
    pub fn to_string_domain(&self) -> String {
        let mut res = Vec::with_capacity(self.0.len());
        let mut pos = 0;
        loop {
            let len = self.0[pos];
            if len == 0 {
                break;
            }
            pos += 1;
            res.extend(&self.0[pos as usize..][..len as usize]);
            res.push(b'.');
            pos += len as usize;
        }
        String::from_utf8(res).unwrap_or_else(|_| "<invalid-domain>".to_string())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for NameOrRdata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("NameOrRdata({:?})", self.0))
    }
}

/// Ticks are sub-second intervals.
///
/// The number of ticks in a second is file/block metadata.
///
/// An unsigned ticks type is available as [`UTicks`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Ticks(i32);

impl fmt::Debug for Ticks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Ticks> for i32 {
    fn from(value: Ticks) -> Self {
        value.0
    }
}

impl From<i32> for Ticks {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

/// A timestamp (two unsigned integers)
///
/// The first integer is the number of seconds since the POSIX epoch, excluding leap seconds.
/// The second integer is the number of ticks since the start of the second.
#[skip_serializing_none]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize_tuple, Deserialize_tuple,
)]
pub struct Timestamp {
    /// Number of seconds since the POSIX epoch.
    pub timestamp_secs: i32,
    /// Number of ticks since the start of the second
    pub timestamp_ticks: UTicks,
}

/// Ticks are sub-second intervals.
///
/// The number of ticks in a second is file/block metadata.
///
/// A signed ticks type is available as [`Ticks`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct UTicks(u32);

impl fmt::Debug for UTicks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<UTicks> for u32 {
    fn from(value: UTicks) -> Self {
        value.0
    }
}

impl From<u32> for UTicks {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

/// A [`RRList`] is an array of unsigned integers, indexes to [`RR`] items in the `rr` array.
pub type RRList = Vec<usize>;

/// A [`QuestionList`] is an array of unsigned integers, indexes to [`Question`] items in the `qrr` array.
pub type QuestionList = Vec<usize>;

// /////////////////////////////////////////////////////////////////////////////
// This section contains the main file structure and preamble
// /////////////////////////////////////////////////////////////////////////////

/// A C-DNS file
///
/// Original format descriptoin in [Section 7.3](https://tools.ietf.org/html/rfc8618#section-7.3)
#[derive(Debug, Serialize_tuple, Deserialize_tuple)]
pub struct File {
    /// String "C-DNS" identifying the file type.
    // TODO assert that deserialization has value "C-DNS"
    pub file_type_id: String,
    /// Version and parameter information for the whole file.
    pub file_preamble: FilePreamble,
    /// Array of items of type [`Block`].
    pub file_blocks: Vec<Block>,
}

/// Information about data in the file.
///
/// Original format description in [Section 7.3.1](https://tools.ietf.org/html/rfc8618#section-7.3.1).
#[skip_serializing_none]
#[derive(Debug, SerializeIndexed, DeserializeIndexed)]
pub struct FilePreamble {
    /// Integer with value `1`.
    ///
    /// The major version of the format used in the file.
    // TODO Assert that deserialization has value 1
    pub major_format_version: u32,
    /// Integer with value `0`.
    ///
    /// The minor version of the format used in the file.
    // TODO Assert that deserialization has value 0
    pub minor_format_version: u32,
    /// Version indicator available for private use by implementations.
    pub private_version: Option<u32>,
    /// Array of items of type [`BlockParameters`].
    ///
    /// The array must contain at least one entry.
    /// (The [`BlockPreamble.block_parameters_index`] item in each [`BlockPreamble`] indicates which array entry applies to that [`Block`].)
    pub block_parameters: Vec<BlockParameters>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

/// Parameters relating to data storage and collection that apply to one or more items of type [`Block`].
///
/// Original format description in [Section 7.3.1.1](https://tools.ietf.org/html/rfc8618#section-7.3.1.1).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
pub struct BlockParameters {
    /// Parameters relating to data storage in a [`Block`] item.
    pub storage_parameters: StorageParameters,
    /// Parameters relating to collection of the data in a [`Block`] item.
    pub collection_parameters: Option<CollectionParameters>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

impl fmt::Debug for BlockParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("StorageParameters");
        ds.field("storage_parameters", &self.storage_parameters);
        crate::debug_unwrap_option_single_field!(self, ds, collection_parameters,);
        ds.finish()
    }
}

/// Parameters relating to how data is stored in the items of type [`Block`]
///
/// Original format description in [Section 7.3.1.1.1](https://tools.ietf.org/html/rfc8618#section-7.3.1.1.1).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct StorageParameters {
    /// Sub-second timing is recorded in ticks.
    ///
    /// This specifies the number of ticks in a second.
    pub ticks_per_second: UTicks,
    /// The maximum number of items stored in any of the arrays in a [`Block`] item (Q/R, Address/Event Count, or Malformed Message data items).
    ///
    /// An indication to a decoder of the resources needed to process the file.
    pub max_block_items: usize,
    /// Collection of hints as to which fields are omitted in the arrays that have optional fields.
    pub storage_hints: StorageHints,
    /// Array of OPCODES (unsigned integers, each in the range 0 to 15 inclusive) recorded by the collecting implementation.
    // TODO assert values 0..15
    pub opcodes: Vec<u8>,
    /// Array of RR TYPEs (unsigned integers, each in the range 0 to 65535 inclusive) recorded by the collecting implementation.
    pub rr_types: Vec<DnsType>,
    /// Bit flags indicating attributes of stored data.
    pub storage_flags: Option<EnumSet<StorageFlags>>,
    /// IPv4 client address prefix length, in the range 1 to 32 inclusive.
    ///
    /// If specified, only the address prefix bits are stored.
    // TODO assert value is 1..32
    pub client_address_prefix_ipv4: Option<u8>,
    /// IPv6 client address prefix length, in the range 1 to 128 inclusive.
    ///
    /// If specified, only the address prefix bits are stored.
    // TODO assert value is 1..128
    pub client_address_prefix_ipv6: Option<u8>,
    /// IPv4 server address prefix length, in the range 1 to 32 inclusive.
    ///
    /// If specified, only the address prefix bits are stored.
    // TODO assert value is 1..32
    pub server_address_prefix_ipv4: Option<u8>,
    /// IPv6 server address prefix length, in the range 1 to 128 inclusive.
    ///
    /// If specified, only the address prefix bits are stored.
    // TODO assert value is 1..128
    pub server_address_prefix_ipv6: Option<u8>,
    /// Information on the sampling method used.
    pub sampling_method: Option<String>,
    /// Information on the anonymization method used.
    pub anonymization_method: Option<String>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

impl fmt::Debug for StorageParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("StorageParameters");
        ds.field("ticks_per_second", &self.ticks_per_second)
            .field("max_block_items", &self.max_block_items)
            .field("storage_hints", &self.storage_hints)
            .field("opcodes", &self.opcodes)
            .field("rr_types", &self.rr_types);
        crate::debug_unwrap_option_single_field!(
            self,
            ds,
            storage_flags,
            client_address_prefix_ipv4,
            client_address_prefix_ipv6,
            server_address_prefix_ipv4,
            server_address_prefix_ipv6,
            sampling_method,
            anonymization_method,
        );
        ds.finish()
    }
}

/// Flag type for [`StorageParameters.storage_flags`]
///
/// * Bit 0. 1 if the data has been anonymized.
/// * Bit 1. 1 if the data is sampled data.
/// * Bit 2. 1 if the names have been normalized (converted to uniform case).
#[derive(Debug, EnumSetType)]
pub enum StorageFlags {
    AnonymizedData = 0,
    SampledData = 1,
    NormalizedNames = 2,
}

/// An indicator of which fields the collecting implementation omits in the maps with optional fields
///
/// Note that hints have a top-down precedence.
/// In other words, where a map contains another map, the hint on the containing map overrides any hints in the contained map and the contained map is omitted.
///
/// Original format description in [Section 7.3.1.1.1.1](https://tools.ietf.org/html/rfc8618#section-7.3.1.1.1.1).
#[derive(Debug, SerializeIndexed, DeserializeIndexed)]
pub struct StorageHints {
    /// Hints indicating which [`QueryResponse`] fields are omitted.
    pub query_response_hints: EnumSet<QueryResponseHints>,
    /// Hints indicating which [`QueryResponseSignature`] fields are omitted.
    pub query_response_signature_hints: EnumSet<QueryResponseSignatureHints>,
    /// Hints indicating which optional [`RR`] fields are omitted.
    pub rr_hints: EnumSet<RRHint>,
    /// Hints indicating which other datatypes are omitted.
    pub other_data_hints: EnumSet<OtherDataHints>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

/// Flag type for [`StorageHints.query_response_hints`]
///
/// Hints indicating which [`QueryResponse`] fields are omitted.
/// If a bit is unset, the field is omitted from the capture.
///
/// * Bit 0. time-offset
/// * Bit 1. client-address-index
/// * Bit 2. client-port
/// * Bit 3. transaction-id
/// * Bit 4. qr-signature-index
/// * Bit 5. client-hoplimit
/// * Bit 6. response-delay
/// * Bit 7. query-name-index
/// * Bit 8. query-size
/// * Bit 9. response-size
/// * Bit 10. response-processing-data
/// * Bit 11. query-question-sections
/// * Bit 12. query-answer-sections
/// * Bit 13. query-authority-sections
/// * Bit 14. query-additional-sections
/// * Bit 15. response-answer-sections
/// * Bit 16. response-authority-sections
/// * Bit 17. response-additional-sections
#[derive(Debug, EnumSetType)]
pub enum QueryResponseHints {
    TimeOffset = 0,
    ClientAddressIndex = 1,
    ClientPort = 2,
    TransactionId = 3,
    QrSignatureIndex = 4,
    ClientHoplimit = 5,
    ResponseDelay = 6,
    QueryNameIndex = 7,
    QuerySize = 8,
    ResponseSize = 9,
    ResponseProcessingData = 10,
    QueryQuestionSections = 11,
    QueryAnswerSections = 12,
    QueryAuthoritySections = 13,
    QueryAdditionalSections = 14,
    ResponseAnswerSections = 15,
    ResponseAuthoritySections = 16,
    ResponseAdditionalSections = 17,
}

/// Flag type for [`StorageHints.query_response_signature_hints`]
///
/// Hints indicating which [`QueryResponseSignature`] fields are omitted.
/// If a bit is unset, the field is omitted from the capture.
///
/// * Bit 0. server-address-index
/// * Bit 1. server-port
/// * Bit 2. qr-transport-flags
/// * Bit 3. qr-type
/// * Bit 4. qr-sig-flags
/// * Bit 5. query-opcode
/// * Bit 6. qr-dns-flags
/// * Bit 7. query-rcode
/// * Bit 8. query-classtype-index
/// * Bit 9. query-qdcount
/// * Bit 10. query-ancount
/// * Bit 11. query-nscount
/// * Bit 12. query-arcount
/// * Bit 13. query-edns-version
/// * Bit 14. query-udp-size
/// * Bit 15. query-opt-rdata-index
/// * Bit 16. response-rcode
#[derive(Debug, EnumSetType)]
pub enum QueryResponseSignatureHints {
    ServerAddressIndex = 0,
    ServerPort = 1,
    QrTransportFlags = 2,
    QrType = 3,
    QrSigFlags = 4,
    QueryOpcode = 5,
    QrDnsFlags = 6,
    QueryRcode = 7,
    QueryClasstypeIndex = 8,
    QueryQdcount = 9,
    QueryAncount = 10,
    QueryNscount = 11,
    QueryArcount = 12,
    QueryEdnsVersion = 13,
    QueryUdpSize = 14,
    QueryOptRdataIndex = 15,
    ResponseRcode = 16,
}

/// Flag type for [`StorageHints.rr_hints`]
///
/// Hints indicating which optional [`RR`] fields are omitted.
/// If a bit is unset, the field is omitted from the capture.
///
/// * Bit 0. ttl
/// * Bit 1. rdata-index
#[derive(Debug, EnumSetType)]
pub enum RRHint {
    Ttl = 0,
    RdataIndex = 1,
}

/// Flag type for [`StorageHints.other_data_hints`]
///
/// Hints indicating which other datatypes are omitted.
/// If a bit is unset, the field is omitted from the capture.
///
/// * Bit 0. malformed-messages
/// * Bit 1. address-event-counts
#[derive(Debug, EnumSetType)]
pub enum OtherDataHints {
    MalformedMessages = 0,
    AddressEventCounts = 1,
}

/// Parameters providing information regarding how data in the file was collected.
///
/// The values are informational only and serve as metadata to downstream analyzers as to the configuration of a collecting implementation.
/// They can provide context when interpreting what data is present/absent from the capture but cannot necessarily be validated against the data captured.
///
/// These parameters have no default.
/// If they do not appear, nothing can be inferred about their value.
///
/// Original format description in [Section 7.3.1.1.2](https://tools.ietf.org/html/rfc8618#section-7.3.1.1.2).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct CollectionParameters {
    /// To be matched with a Query, a Response must arrive within this number of milliseconds.
    pub query_timeout: Option<u32>,
    /// The network stack may report a Response before the corresponding Query.
    ///
    /// A Response is not considered to be missing a Query until after this many microseconds.
    pub skew_timeout: Option<u32>,
    /// Collect up to this many bytes per packet.
    pub snaplen: Option<u32>,
    /// `true` if promiscuous mode was enabled on the interface, `false` otherwise.
    pub promisc: Option<bool>,
    /// Array of identifiers (of type text string) of the interfaces used for collection.
    pub interfaces: Option<Vec<String>>,
    /// Array of server collection IP addresses (of type byte string).
    ///
    /// Metadata for downstream analyzers; does not affect collection.
    pub server_addresses: Option<Vec<IpAddr>>,
    /// Array of identifiers (of type unsigned integer, each in the range 1 to 4094 inclusive) of VLANs IEEE802.1Q selected for collection.
    ///
    /// VLAN IDs are unique only within an administrative domain.
    // TODO assert values 1..4094
    pub vlan_ids: Option<u16>,
    /// Filter for input, in "tcpdump" pcap-filter style.
    pub filter: Option<String>,
    /// Implementation-specific human-readable string identifying the collection method.
    pub generator_id: Option<String>,
    /// String identifying the collecting host.
    pub host_id: Option<String>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

crate::debug_unwrap_option_fields!(
    CollectionParameters,
    query_timeout,
    skew_timeout,
    snaplen,
    promisc,
    interfaces,
    server_addresses,
    vlan_ids,
    filter,
    generator_id,
    host_id,
);

/// Container for data with common collection and storage parameters.
///
/// Original format description in [Section 7.3.2](https://tools.ietf.org/html/rfc8618#section-7.3.2).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct Block {
    /// Overall information for the [`Block`] item.
    pub block_preamble: BlockPreamble,
    /// Statistics about the [`Block`] item.
    pub block_statistics: Option<BlockStatistics>,
    /// The arrays containing data referenced by individual [`QueryResponse`] or [`MalformedMessage`] items.
    pub block_tables: Option<BlockTables>,
    /// Details of individual C-DNS Q/R data items.
    pub query_responses: Option<Vec<QueryResponse>>,
    /// Per-client counts of ICMP messages and TCP resets.
    pub address_event_counts: Option<Vec<AddressEventCount>>,
    /// Details of malformed DNS messages.
    pub malformed_messages: Option<Vec<MalformedMessage>>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("Block");
        ds.field("block_preamble", &self.block_preamble);
        crate::debug_unwrap_option_single_field!(
            self,
            ds,
            block_statistics,
            block_tables,
            query_responses,
            address_event_counts,
            malformed_messages,
        );
        ds.finish()
    }
}

/// Overall information for a "Block" item.
///
/// Original format description in [Section 7.3.2.1](https://tools.ietf.org/html/rfc8618#section-7.3.2.1).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
pub struct BlockPreamble {
    /// A timestamp for the earliest record in the [`Block`] item.
    ///
    /// This field is mandatory unless all block items containing a time offset from the start of the [`Block`] also omit that time offset.
    pub earliest_time: Option<Timestamp>,
    /// The index of the item in the [`FilePreamble.block_parameters`] array applicable to this block.
    ///
    /// If not present, index 0 is used.
    pub block_parameters_index: Option<usize>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

crate::debug_unwrap_option_fields!(BlockPreamble, earliest_time, block_parameters_index,);

/// Basic statistical information about a [`Block`] item.
///
/// Original format description in [Section 7.3.2.2](https://tools.ietf.org/html/rfc8618#section-7.3.2.2).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct BlockStatistics {
    /// Total number of well-formed DNS messages processed from the input traffic stream during collection of data in this [`Block`] item.
    pub processed_messages: Option<usize>,
    /// Total number of Q/R data items in this [`Block`] item.
    pub qr_data_items: Option<usize>,
    /// Number of unmatched Queries in this [`Block`] item.
    pub unmatched_queries: Option<usize>,
    /// Number of unmatched Responses in this [`Block`] item.
    pub unmatched_responses: Option<usize>,
    /// Number of DNS messages processed from the input traffic stream during collection of data in this [`Block`] item but not recorded because their OPCODE is not in the list to be collected.
    pub discarded_opcode: Option<u8>,
    /// Number of malformed messages processed from the input traffic stream during collection of data in this [`Block`] item.
    pub malformed_items: Option<usize>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

crate::debug_unwrap_option_fields!(
    BlockStatistics,
    processed_messages,
    qr_data_items,
    unmatched_queries,
    unmatched_responses,
    discarded_opcode,
    malformed_items,
);

/// Map of arrays containing data referenced by individual [`QueryResponse`] or [`MalformedMessage`] items in this [`Block`].
///
/// Each element is an array that, if present, must not be empty.
///
/// An item in the `qlist` array contains indexes to values in the `qrr` array.
/// Therefore, if `qlist` is present, `qrr` must also be present.
/// Similarly, if `rrlist` is present, `rr` must also be present.
///
/// Original format description in [Section 7.3.2.3](https://tools.ietf.org/html/rfc8618#section-7.3.2.3).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct BlockTables {
    /// Array of IP addresses, in network byte order (of type byte string).
    ///
    /// If client or server address prefixes are set, only the address prefix bits are stored.
    /// Each string is therefore up to 4 bytes long for an IPv4 address, or up to 16 bytes long for an IPv6 address.
    pub ip_address: Option<Vec<IpAddr>>,
    /// Array of RR CLASS and TYPE information.
    pub classtype: Option<Vec<ClassType>>,
    /// Array where each entry is the contents of a single NAME or RDATA in wire format (of type byte string).
    ///
    /// Note that NAMEs, and labels within RDATA contents, are full domain names or labels; no name compression is used on the individual names/labels within the format.
    pub name_rdata: Option<Vec<NameOrRdata>>,
    /// Array of Q/R data item signatures.
    pub qr_sig: Option<Vec<QueryResponseSignature>>,

    // These two fields are inlined from QuestionTables
    // question_tables: Option<QuestionTables>,
    /// Array of type "QuestionList".
    ///
    /// A [`QuestionList`] is an array of unsigned integers, indexes to [`Question`] items in the `qrr` array.
    pub qlist: Option<Vec<QuestionList>>,
    /// Array of type "Question".
    ///
    /// Each entry is the contents of a single Question, where a Question is the second or subsequent Question in a Query.
    pub qrr: Option<Vec<Question>>,

    // These two fields are inlined from RRTables
    // rr_tables: Option<RRTables>,
    /// Array of type [`RRList`].
    ///
    /// An [`RRList`] is an array of unsigned integers, indexes to [`RR`] items in the `rr` array.
    pub rrlist: Option<Vec<RRList>>,
    /// Array of type [`RR`].
    ///
    /// Each entry is the contents of a single [`RR`].
    pub rr: Option<Vec<RR>>,

    /// Array of the contents of malformed messages.
    pub malformed_message_data: Option<Vec<MalformedMessageData>>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

crate::debug_unwrap_option_fields!(
    BlockTables,
    ip_address,
    classtype,
    name_rdata,
    qr_sig,
    qlist,
    qrr,
    rrlist,
    rr,
    malformed_message_data,
);

/// RR CLASS and TYPE information.
///
/// Original format description in [Section 7.3.2.3.1](https://tools.ietf.org/html/rfc8618#section-7.3.2.3.1).
#[derive(SerializeIndexed, DeserializeIndexed)]
pub struct ClassType {
    /// TYPE value.
    pub type_: DnsType,
    /// CLASS value.
    pub class: DnsClass,
}

impl fmt::Debug for ClassType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        /* OPT */
        if self.type_ == DnsType(41) {
            f.write_fmt(format_args!("OPT (UDP Size: {})", u16::from(self.class)))
        } else {
            f.write_fmt(format_args!("{:?} {:?}", self.type_, self.class))
        }
    }
}

// TODO some fields serialize in a different order than compactor
//
// This is the order of some of the fields
// 2: 1
// 6: 129
// 4: f
// 9: 1
// 8: 0
// 7: 0
// 5: 0
// a: 0
// c: 1
// b: 0
// d: 0

/// Elements of a Q/R data item that are often common between multiple individual Q/R data items.
///
/// Original format description in [Section 7.3.2.3.2](https://tools.ietf.org/html/rfc8618#section-7.3.2.3.2).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
pub struct QueryResponseSignature {
    /// The index in the [`BlockTables.ip_address`] array of the server IP address.
    pub server_address_index: Option<usize>,
    /// The server port.
    pub server_port: Option<u16>,
    /// Bit flags describing the transport used to service the [`Query`].
    pub qr_transport_flags: Option<TransportFlags>,
    /// Type of Query/Response transaction based on the definitions in the dnstap schema.
    pub qr_type: Option<QueryResponseType>,
    /// Bit flags explicitly indicating attributes of the message pair represented by this Q/R data item (not all attributes may be recorded or deducible).
    pub qr_sig_flags: Option<EnumSet<QueryResponseFlags>>,
    /// Query OPCODE.
    pub query_opcode: Option<u8>,
    /// Bit flags with values from the Query and Response DNS flags.
    ///
    /// Flag values are 0 if the Query or Response is not present.
    pub qr_dns_flags: Option<EnumSet<DNSFlags>>,
    /// Query RCODE.
    ///
    /// If the Query contains an OPT RR RFC6891, this value incorporates any EXTENDED-RCODE value.
    pub query_rcode: Option<u16>,
    /// The index in the [`BlockTables.classtype`] array of the CLASS and TYPE of the first Question.
    pub query_classtype_index: Option<usize>,
    /// The QDCOUNT in the Query, or Response if no Query present.
    pub query_qdcount: Option<usize>,
    /// Query ANCOUNT.
    pub query_ancount: Option<usize>,
    /// Query NSCOUNT.
    pub query_nscount: Option<usize>,
    /// Query ARCOUNT.
    pub query_arcount: Option<usize>,
    /// The Query EDNS version.
    pub query_edns_version: Option<u8>,
    /// The Query EDNS sender's UDP payload size.
    pub query_udp_size: Option<u16>,
    /// The index in the [`BlockTables.name_rdata`] array of the OPT RDATA.
    pub query_opt_rdata_index: Option<usize>,
    /// Response RCODE.
    ///
    /// If the Response contains an OPT RR, this value incorporates any EXTENDED-RCODE value.
    pub response_rcode: Option<u16>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

crate::debug_unwrap_option_fields!(
    QueryResponseSignature,
    server_address_index,
    server_port,
    qr_transport_flags,
    qr_type,
    qr_sig_flags,
    query_opcode,
    qr_dns_flags,
    query_rcode,
    query_classtype_index,
    query_qdcount,
    query_ancount,
    query_nscount,
    query_arcount,
    query_edns_version,
    query_udp_size,
    query_opt_rdata_index,
    response_rcode,
);

/// Bit flags describing the transport used to service the Query.
///
/// * Bit 0. IP version.  0 if IPv4, 1 if IPv6.
/// * Bits 1-4. Transport.  4-bit unsigned value where
///     * 0 = UDP RFC 1035
///     * 1 = TCP RFC 1035
///     * 2 = TLS RFC 7858
///     * 3 = DTLS RFC 8094
///     * 4 = HTTPS RFC 8484
///     * 15 = Non-standard transport (see below)
///     * Values 5-14 are reserved for future use.
/// * Bit 5. `1` if trailing bytes in Query packet.
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransportFlags(u8);

impl TransportFlags {
    pub fn is_ipv4(&self) -> bool {
        self.0 & 0b0000_0001 == 0
    }

    pub fn is_ipv6(&self) -> bool {
        !self.is_ipv4()
    }

    pub fn transport_protocol(&self) -> crate::Transport {
        // Bit 1..=4 are for Transport
        let transport = (self.0 & 0b0001_1110) >> 3;
        match transport {
            0 => crate::Transport::Udp,
            1 => crate::Transport::Tcp,
            2 => crate::Transport::Tls,
            3 => crate::Transport::Dtls,
            4 => crate::Transport::Https,
            15 => crate::Transport::NonStandard,
            _ => crate::Transport::Reserved,
        }
    }

    pub fn has_trailing_data(&self) -> bool {
        self.0 & 0b0010_0000 != 0
    }
}

impl fmt::Debug for TransportFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // First bit of TransportFlagValues is ip-version
        if self.is_ipv4() {
            f.write_str("IPv4")?;
        } else {
            f.write_str("IPv6")?;
        }

        f.write_str(match self.transport_protocol() {
            crate::Transport::Udp => " | UDP",
            crate::Transport::Tcp => " | TCP",
            crate::Transport::Tls => " | TLS",
            crate::Transport::Dtls => " | DTLS",
            crate::Transport::Https => " | HTTPS",
            crate::Transport::Reserved => " | Reserved",
            crate::Transport::NonStandard => " | Non-Standard",
        })?;

        if self.has_trailing_data() {
            f.write_str(" | Query has trailing data")?;
        }
        Ok(())
    }
}

/// Type of Query/Response transaction based on the definitions in the dnstap schema
///
/// The dnstap schema is hosted in this repository:
/// <https://github.com/dnstap/dnstap.pb/blob/master/dnstap.proto>
#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[serde(deny_unknown_fields)]
#[repr(u8)]
pub enum QueryResponseType {
    /// A transaction between a stub resolver and a DNS server from the perspective of the stub resolver.
    Stub = 0,
    /// A transaction between a client and a DNS server (a proxy or full recursive resolver) from the perspective of the DNS server.
    Client = 1,
    /// A transaction between a recursive resolver and an authoritative server from the perspective of the recursive resolver.
    Resolver = 2,
    /// A transaction between a recursive resolver and an authoritative server from the perspective of the authoritative server.
    Authoritative = 3,
    /// A transaction between a downstream forwarder and an upstream DNS server (a recursive resolver) from the perspective of the downstream forwarder.
    Forwarder = 4,
    /// A transaction between a DNS software tool and a DNS server, from the perspective of the tool.
    Tool = 5,
}

/// Bit flags explicitly indicating attributes of the message pair represented by this Q/R data item (not all attributes may be recorded or deducible).
///
/// * Bit 0. 1 if a Query was present.
/// * Bit 1. 1 if a Response was present.
/// * Bit 2. 1 if a Query was present and it had an OPT RR.
/// * Bit 3. 1 if a Response was present and it had an OPT RR.
/// * Bit 4. 1 if a Query was present but had no Question.
/// * Bit 5. 1 if a Response was present but had no Question (only one query-name-index is stored per Q/R data item).
#[derive(Debug, EnumSetType)]
pub enum QueryResponseFlags {
    HasQuery = 0,
    HasResponse = 1,
    QueryHasOpt = 2,
    ResponseHasOpt = 3,
    QueryHasNoQuestion = 4,
    ResponseHasNoQuestion = 5,
}

/// Bit flags with values from the Query and Response DNS flags.
///
/// Flag values are 0 if the Query or Response is not present.
///
/// * Bit 0. Query Checking Disabled (CD).
/// * Bit 1. Query Authenticated Data (AD).
/// * Bit 2. Query reserved (Z).
/// * Bit 3. Query Recursion Available (RA).
/// * Bit 4. Query Recursion Desired (RD).
/// * Bit 5. Query TrunCation (TC).
/// * Bit 6. Query Authoritative Answer (AA).
/// * Bit 7. Query DNSSEC answer OK (DO).
/// * Bit 8. Response Checking Disabled (CD).
/// * Bit 9. Response Authenticated Data (AD).
/// * Bit 10. Response reserved (Z).
/// * Bit 11. Response Recursion Available (RA).
/// * Bit 12. Response Recursion Desired (RD).
/// * Bit 13. Response TrunCation (TC).
/// * Bit 14. Response Authoritative Answer (AA).
#[derive(Debug, EnumSetType)]
pub enum DNSFlags {
    QueryCd = 0,
    QueryAd = 1,
    QueryZ = 2,
    QueryRa = 3,
    QueryRd = 4,
    QueryTc = 5,
    QueryAa = 6,
    QueryDo = 7,
    ResponseCd = 8,
    ResponseAd = 9,
    ResponseZ = 10,
    ResponseRa = 11,
    ResponseRd = 12,
    ResponseRc = 13,
    ResponseAa = 14,
}

/// Details on individual Questions in a Question section.
///
/// Original format description in [Section 7.3.2.3.3](https://tools.ietf.org/html/rfc8618#section-7.3.2.3.3).
#[derive(Debug, SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct Question {
    /// The index in the [`BlockTables.name_rdata`] array of the QNAME.
    pub name_index: usize,
    /// The index in the [`BlockTables.classtype`] array of the CLASS and TYPE of the Question.
    pub classtype_index: usize,
}

/// Details on individual RRs in RR sections.
///
/// Original format description in [Section 7.3.2.3.4](https://tools.ietf.org/html/rfc8618#section-7.3.2.3.4).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
pub struct RR {
    /// The index in the [`BlockTables.name_rdata`] array of the NAME.
    pub name_index: usize,
    /// The index in the [`BlockTables.classtype`] array of the CLASS and TYPE of the RR.
    pub classtype_index: usize,
    /// The RR Time to Live.
    pub ttl: Option<u32>,
    /// The index in the [`BlockTables.name_rdata`] array of the RR RDATA.
    pub rdata_index: Option<usize>,
}

impl fmt::Debug for RR {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("RR");
        ds.field("name_index", &self.name_index)
            .field("classtype_index", &self.classtype_index);
        crate::debug_unwrap_option_single_field!(self, ds, ttl, rdata_index,);
        ds.finish()
    }
}

/// Details on malformed DNS messages stored in this [`Block`] item.
///
/// Original format description in [Section 7.3.2.3.5](https://tools.ietf.org/html/rfc8618#section-7.3.2.3.5).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct MalformedMessageData {
    /// The index in the [`BlockTables.ip_address`] array of the server IP address.
    pub server_address_index: Option<usize>,
    /// The server port.
    pub server_port: Option<u16>,
    /// Bit flags describing the transport used to service the Query.
    pub mm_transport_flags: Option<TransportFlags>,
    /// The payload (raw bytes) of the DNS message.
    pub mm_payload: Option<ByteBuf>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

crate::debug_unwrap_option_fields!(
    MalformedMessageData,
    server_address_index,
    server_port,
    mm_transport_flags,
    mm_payload,
);

/// Details on individual Q/R data items.
///
/// Note that there is no requirement that the elements of the [`BlockTables.query_responses`] array are presented in strict chronological order.
///
/// The `query_size` and `response_size fields hold the DNS message size.
/// For UDP, this is the size of the UDP payload that contained the DNS message.
/// For TCP, it is the size of the DNS message as specified in the two-byte message length header.
/// Trailing bytes in UDP Queries are routinely observed in traffic to authoritative servers, and this value allows a calculation of how many trailing bytes were present.
///
/// Original format description in [Section 7.3.2.4](https://tools.ietf.org/html/rfc8618#section-7.3.2.4).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct QueryResponse {
    /// Q/R timestamp as an offset in ticks from [`BlockPreamble.earliest_time`].
    ///
    /// The timestamp is the timestamp of the Query, or the Response if there is no Query.
    pub time_offset: Option<UTicks>,
    /// The index in the [`BlockTables.ip_address`] array of the client IP address.
    pub client_address_index: Option<usize>,
    /// The client port.
    pub client_port: Option<u16>,
    /// DNS transaction identifier.
    pub transaction_id: Option<u16>,
    /// The index in the [`BlockTables.qr_sig`] array of the [`QueryResponseSignature`] item.
    pub qr_signature_index: Option<usize>,
    /// The IPv4 TTL or IPv6 Hoplimit from the Query packet.
    pub client_hoplimit: Option<u8>,
    /// The time difference between Query and Response, in ticks.
    ///
    /// Only present if there is a Query and a Response.
    /// The delay can be negative if the network stack/capture library returns packets out of order.
    pub response_delay: Option<Ticks>,
    /// The index in the [`BlockTables.name_rdata`] array of the item containing the QNAME for the first Question.
    pub query_name_index: Option<usize>,
    /// DNS Query message size.
    pub query_size: Option<u16>,
    /// DNS Response message size.
    pub response_size: Option<u16>,
    /// Data on Response processing.
    pub response_processing_data: Option<ResponseProcessingData>,
    /// Extended Query data.
    pub query_extended: Option<QueryResponseExtended>,
    /// Extended Response data.
    pub response_extended: Option<QueryResponseExtended>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

crate::debug_unwrap_option_fields!(
    QueryResponse,
    time_offset,
    client_address_index,
    client_port,
    transaction_id,
    qr_signature_index,
    client_hoplimit,
    response_delay,
    query_name_index,
    query_size,
    response_size,
    response_processing_data,
    query_extended,
    response_extended,
);

/// Information on the server processing that produced the Response.
///
/// Original format description in [Section 7.3.2.4.1](https://tools.ietf.org/html/rfc8618#section-7.3.2.4.1).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct ResponseProcessingData {
    /// The index in the [`BlockTables.name_rdata`] array of the owner name for the Response bailiwick.
    pub bailiwick_index: Option<usize>,
    /// Flags relating to Response processing.
    pub processing_flags: Option<ResponseProcessingFlags>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

crate::debug_unwrap_option_fields!(ResponseProcessingData, bailiwick_index, processing_flags,);

/// Flags relating to Response processing.
///
/// * Bit 0. 1 if the Response came from cache.
#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ResponseProcessingFlags {
    FromCache = 0,
}

/// Extended data on the Q/R data item.
///
/// Each item in the map is present only if collection of the relevant details is configured
/// Information on the server processing that produced the Response.
///
/// Original format description in [Section 7.3.2.4.2](https://tools.ietf.org/html/rfc8618#section-7.3.2.4.2).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct QueryResponseExtended {
    /// The index in the [`BlockTables.qlist`] array of the entry listing any second and subsequent Questions in the Question section for the Query or Response.
    pub question_index: Option<usize>,
    /// The index in the [`BlockTables.rrlist`] array of the entry listing the Answer RR sections for the Query or Response.
    pub answer_index: Option<usize>,
    /// The index in the [`BlockTables.rrlist`] array of the entry listing the Authority RR sections for the Query or Response.
    pub authority_index: Option<usize>,
    /// The index in the [`BlockTables.rrlist`] array of the entry listing the Additional RR sections for the Query or Response.
    ///
    ///  Note that Query OPT RR data can optionally be stored in the QuerySignature.
    pub additional_index: Option<usize>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

crate::debug_unwrap_option_fields!(
    QueryResponseExtended,
    question_index,
    answer_index,
    authority_index,
    additional_index,
);

/// Counts of various IP-related events relating to traffic with individual client addresses.
///
/// Original format description in [Section 7.3.2.5](https://tools.ietf.org/html/rfc8618#section-7.3.2.5).
#[skip_serializing_none]
#[derive(SerializeIndexed, DeserializeIndexed)]
pub struct AddressEventCount {
    /// The type of event.
    pub ae_type: AddressEventType,
    /// A code relating to the event.
    ///
    /// For ICMP or ICMPv6 events, this MUST be the ICMP (RFC 792) or ICMPv6 (RFC 4443) code.
    /// For other events, the contents are undefined.
    pub ae_code: Option<u32>,
    /// The index in the [`BlockTables.ip_address`] array of the client address.
    pub ae_address_index: usize,
    /// Bit flags describing the transport used to service the event.
    pub ae_transport_flags: Option<TransportFlags>,
    /// The number of occurrences of this event during the [`Block`] collection period.
    pub ae_count: usize,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}

impl fmt::Debug for AddressEventCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("AddressEventCount");
        ds.field("ae_type", &self.ae_type);
        crate::debug_unwrap_option_single_field!(self, ds, ae_code,);
        ds.field("ae_address_index", &self.ae_address_index);
        crate::debug_unwrap_option_single_field!(self, ds, ae_transport_flags,);
        ds.field("ae_count", &self.ae_type);
        ds.finish()
    }
}

/// The type of event.
///
/// * `0`: TCP reset.
/// * `1`: ICMP time exceeded.
/// * `2`: ICMP destination unreachable.
/// * `3`: ICMPv6 time exceeded.
/// * `4`: ICMPv6 destination unreachable.
/// * `5`: ICMPv6 packet too big.
#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum AddressEventType {
    TcpReset = 0,
    IcmpTimeExceeded = 1,
    IcmpDestinationUnreachable = 2,
    Icmpv6TimeExceeded = 3,
    Icmpv6DestinationUnreachable = 4,
    Icmpv6PacketTooBig = 5,
}

/// Details on Malformed Message data items.
///
/// Original format description in [Section 7.3.2.6](https://tools.ietf.org/html/rfc8618#section-7.3.2.6).
#[skip_serializing_none]
#[derive(Debug, SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(emit_length = false)]
pub struct MalformedMessage {
    /// Message timestamp as an offset in ticks from [`BlockPreamble.earliest_time`].
    pub time_offset: Option<UTicks>,
    /// The index in the [`BlockTables.ip_address`] array of the client IP address.
    pub client_address_index: Option<usize>,
    /// The client port.
    pub client_port: Option<u16>,
    /// The index in the [`BlockTables.malformed_message_data`] array of the message data for this message.
    pub message_data_index: Option<usize>,

    /// Collect additional custom values with negative index values.
    #[serde_indexed(extras)]
    pub extra_values: BTreeMap<isize, serde_cbor::Value>,
}
