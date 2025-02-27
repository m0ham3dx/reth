//! All capability related types

use crate::{version::ParseVersionError, EthMessage, EthVersion};
use bytes::{BufMut, Bytes};
use reth_rlp::{Decodable, DecodeError, Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// A Capability message consisting of the message-id and the payload
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawCapabilityMessage {
    /// Identifier of the message.
    pub id: usize,
    /// Actual payload
    pub payload: Bytes,
}

/// Various protocol related event types bubbled up from a session that need to be handled by the
/// network.
#[derive(Debug, Serialize, Deserialize)]
pub enum CapabilityMessage {
    /// Eth sub-protocol message.
    Eth(EthMessage),
    /// Any other capability message.
    Other(RawCapabilityMessage),
}

/// A message indicating a supported capability and capability version.
#[derive(
    Clone, Debug, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize, Default, Hash,
)]
pub struct Capability {
    /// The name of the subprotocol
    pub name: SmolStr,
    /// The version of the subprotocol
    pub version: usize,
}

impl Capability {
    /// Create a new `Capability` with the given name and version.
    pub fn new(name: SmolStr, version: usize) -> Self {
        Self { name, version }
    }

    /// Whether this is eth v66 protocol.
    #[inline]
    pub fn is_eth_v66(&self) -> bool {
        self.name == "eth" && self.version == 66
    }

    /// Whether this is eth v67.
    #[inline]
    pub fn is_eth_v67(&self) -> bool {
        self.name == "eth" && self.version == 67
    }
}

/// Represents all capabilities of a node.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Capabilities {
    /// All Capabilities and their versions
    inner: Vec<Capability>,
    eth_66: bool,
    eth_67: bool,
}

impl Capabilities {
    /// Returns all capabilities.
    #[inline]
    pub fn capabilities(&self) -> &[Capability] {
        &self.inner
    }

    /// Consumes the type and returns the all capabilities.
    #[inline]
    pub fn into_inner(self) -> Vec<Capability> {
        self.inner
    }

    /// Whether the peer supports `eth` sub-protocol.
    #[inline]
    pub fn supports_eth(&self) -> bool {
        self.eth_67 || self.eth_66
    }

    /// Whether this peer supports eth v66 protocol.
    #[inline]
    pub fn supports_eth_v66(&self) -> bool {
        self.eth_66
    }

    /// Whether this peer supports eth v67 protocol.
    #[inline]
    pub fn supports_eth_v67(&self) -> bool {
        self.eth_67
    }
}

impl From<Vec<Capability>> for Capabilities {
    fn from(value: Vec<Capability>) -> Self {
        Self {
            eth_66: value.iter().any(Capability::is_eth_v66),
            eth_67: value.iter().any(Capability::is_eth_v67),
            inner: value,
        }
    }
}

impl Encodable for Capabilities {
    fn encode(&self, out: &mut dyn BufMut) {
        self.inner.encode(out)
    }
}

impl Decodable for Capabilities {
    fn decode(buf: &mut &[u8]) -> Result<Self, DecodeError> {
        let inner = Vec::<Capability>::decode(buf)?;

        Ok(Self {
            eth_66: inner.iter().any(Capability::is_eth_v66),
            eth_67: inner.iter().any(Capability::is_eth_v67),
            inner,
        })
    }
}

/// This represents a shared capability, its version, and its offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum SharedCapability {
    /// The `eth` capability.
    Eth { version: EthVersion, offset: u8 },

    /// An unknown capability.
    UnknownCapability { name: SmolStr, version: u8, offset: u8 },
}

impl SharedCapability {
    /// Creates a new [`SharedCapability`] based on the given name, offset, and version.
    pub(crate) fn new(name: &str, version: u8, offset: u8) -> Result<Self, SharedCapabilityError> {
        match name {
            "eth" => Ok(Self::Eth { version: EthVersion::try_from(version)?, offset }),
            _ => Ok(Self::UnknownCapability { name: name.into(), version, offset }),
        }
    }

    /// Returns the name of the capability.
    pub fn name(&self) -> &str {
        match self {
            SharedCapability::Eth { .. } => "eth",
            SharedCapability::UnknownCapability { name, .. } => name,
        }
    }

    /// Returns the version of the capability.
    pub fn version(&self) -> u8 {
        match self {
            SharedCapability::Eth { version, .. } => *version as u8,
            SharedCapability::UnknownCapability { version, .. } => *version,
        }
    }

    /// Returns the message ID offset of the current capability.
    pub fn offset(&self) -> u8 {
        match self {
            SharedCapability::Eth { offset, .. } => *offset,
            SharedCapability::UnknownCapability { offset, .. } => *offset,
        }
    }

    /// Returns the number of protocol messages supported by this capability.
    pub fn num_messages(&self) -> Result<u8, SharedCapabilityError> {
        match self {
            SharedCapability::Eth { version, .. } => Ok(version.total_messages()),
            _ => Err(SharedCapabilityError::UnknownCapability),
        }
    }
}

/// An error that may occur while creating a [`SharedCapability`].
#[derive(Debug, thiserror::Error)]
pub enum SharedCapabilityError {
    /// Unsupported `eth` version.
    #[error(transparent)]
    UnsupportedVersion(#[from] ParseVersionError),
    /// Cannot determine the number of messages for unknown capabilities.
    #[error("cannot determine the number of messages for unknown capabilities")]
    UnknownCapability,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_eth_67() {
        let capability = SharedCapability::new("eth", 67, 0).unwrap();

        assert_eq!(capability.name(), "eth");
        assert_eq!(capability.version(), 67);
        assert_eq!(capability, SharedCapability::Eth { version: EthVersion::Eth67, offset: 0 });
    }

    #[test]
    fn from_eth_66() {
        let capability = SharedCapability::new("eth", 66, 0).unwrap();

        assert_eq!(capability.name(), "eth");
        assert_eq!(capability.version(), 66);
        assert_eq!(capability, SharedCapability::Eth { version: EthVersion::Eth66, offset: 0 });
    }
}
