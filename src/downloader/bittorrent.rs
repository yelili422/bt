use core::panic;
use std::collections::HashMap;

use serde_bencode::value::Value as BencodeValue;
use sha1::Digest;

#[derive(Debug, Clone)]
pub struct Torrent {
    _raw: Vec<u8>,
    _val: BencodeValue,
}

impl Torrent {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_bencode::Error> {
        let root: BencodeValue = serde_bencode::from_bytes(bytes)?;
        match &root {
            BencodeValue::Dict(val) => match val.get("info".as_bytes()) {
                Some(BencodeValue::Dict(_)) => {}
                _ => return Err(serde_bencode::Error::Custom("info field not found".to_string())),
            },
            _ => return Err(serde_bencode::Error::Custom("root value is not a dict".to_string())),
        }

        Ok(Self {
            _raw: bytes.to_vec(),
            _val: root,
        })
    }

    fn get_info(&self) -> Option<&BencodeValue> {
        match &self._val {
            BencodeValue::Dict(val) => val.get("info".as_bytes()),
            _ => None,
        }
    }

    pub fn info_hash(&self) -> [u8; 20] {
        match self.get_info() {
            Some(BencodeValue::Dict(info)) => {
                let info_formatted: HashMap<String, BencodeValue> = info
                    .iter()
                    .map(|(k, v)| (String::from_utf8(k.to_vec()).unwrap(), v.clone()))
                    .collect();

                let info_encoded =
                    serde_bencode::to_bytes(&info_formatted).expect("failed to encode info");

                let mut hasher = sha1::Sha1::new();
                hasher.update(&info_encoded);
                hasher.finalize().try_into().expect("hash length is not 20")
            }
            _ => unreachable!(),
        }
    }

    pub fn get_info_name(&self) -> String {
        match self.get_info() {
            Some(BencodeValue::Dict(info)) => match info.get("name".as_bytes()) {
                Some(BencodeValue::Bytes(name)) => String::from_utf8(name.to_vec()).unwrap(),
                _ => panic!("invalid name field"),
            },
            _ => unreachable!(""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info_hash() {
        let dot_torrent =
            std::fs::read("tests/dataset/872ab5abd72ea223d2a2e36688cc96f83bb71d42.torrent")
                .unwrap();
        let torrent = Torrent::from_bytes(&dot_torrent).unwrap();
        let info_hash = torrent.info_hash();

        assert_eq!(hex::encode(info_hash), "872ab5abd72ea223d2a2e36688cc96f83bb71d42");
    }

    #[test]
    fn test_info_hash_v2() {
        let dot_torrent =
            std::fs::read("tests/dataset/0525f17ac5a68d0198812597747579be78053112.torrent")
                .unwrap();
        let torrent = Torrent::from_bytes(&dot_torrent).unwrap();
        let info_hash = torrent.info_hash();

        assert_eq!(hex::encode(info_hash), "0525f17ac5a68d0198812597747579be78053112");
    }
}
