use minicbor::{Decode, Encode};
use ockam::identity::Identifier;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddMember {
    #[n(1)] member: Identifier,
    #[b(2)] attributes: HashMap<String, String>,
}

impl AddMember {
    pub fn new(member: Identifier) -> Self {
        AddMember {
            member,
            attributes: HashMap::new(),
        }
    }

    pub fn with_attributes(mut self, attributes: HashMap<String, String>) -> Self {
        self.attributes = attributes
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    pub fn member(&self) -> &Identifier {
        &self.member
    }

    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }
}

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateToken {
    #[b(1)] attributes: HashMap<String, String>,
    #[b(2)] token_duration_secs: Option<u64>
}

impl CreateToken {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        CreateToken {
            attributes: HashMap::new(),
            token_duration_secs: None,
        }
    }

    pub fn with_attributes(mut self, attributes: HashMap<String, String>) -> Self {
        self.attributes = attributes;
        self
    }

    pub fn with_duration(mut self, token_duration: Option<Duration>) -> Self {
        self.token_duration_secs = token_duration.map(|d| d.as_secs());
        self
    }

    pub fn attributes(self) -> HashMap<String, String> {
        self.attributes
    }

    pub fn token_duration(&self) -> Option<Duration> {
        self.token_duration_secs.map(Duration::from_secs)
    }
}
