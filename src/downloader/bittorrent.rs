/// These code comes from [Jon Gjengset's stream](https://www.youtube.com/watch?v=jf_ddGnum_4&list=LL&index=4&t=4834s&ab_channel=JonGjengset)
use serde::{Deserialize, Serialize};
use sha1::Digest;

pub use hashes::Hashes;

/// The torrent file structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    /// The URL of the tracker
    pub announce: String,
    pub info: TorrentInfo,
}

impl Torrent {
    pub fn info_hash(&self) -> [u8; 20] {
        let info_encoded = serde_bencode::to_bytes(&self.info).expect("failed to encode info");

        let mut hasher = sha1::Sha1::new();
        hasher.update(&info_encoded);
        hasher.finalize().try_into().expect("hash length is not 20")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TorrentInfo {
    /// The suggested name to save the file/directory
    pub name: String,

    /// The number of bytes in each piece the file is split into
    #[serde(rename = "piece length")]
    pub piece_length: u64,

    /// Each entry of `pieces` is the SHA1 hash of each piece
    pub pieces: Hashes,

    /// The length of the file in bytes for single-file torrents
    pub length: Option<u64>,

    /// For the purposes of the other keys in `Info`, the multi-file case is treated as only having
    /// a single file by concatenating the files in the order they appear in the files list.
    pub files: Option<Vec<File>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    /// The length of the file, in bytes.
    pub length: usize,

    /// Subdirectory names for this file, the last of which is the actual file name
    /// (a zero length list is an error case).
    pub path: Vec<String>,
}

mod hashes {
    use serde::Serialize;

    #[derive(Debug)]
    pub struct Hashes(Vec<[u8; 20]>);

    struct HashesVisitor;

    impl<'de> serde::de::Visitor<'de> for HashesVisitor {
        type Value = Hashes;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string of length multiple of 20")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v.len() % 20 != 0 {
                return Err(serde::de::Error::invalid_length(v.len(), &"a multiple of 20"));
            }
            Ok(Hashes(
                v.chunks_exact(20)
                    .map(|slice_20| slice_20.try_into().expect("guaranteed to be length 20"))
                    .collect(),
            ))
        }
    }

    impl<'de> serde::de::Deserialize<'de> for Hashes {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::de::Deserializer<'de>,
        {
            deserializer.deserialize_bytes(HashesVisitor)
        }
    }

    impl Serialize for Hashes {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::ser::Serializer,
        {
            let single_slice = self.0.concat();
            serializer.serialize_bytes(&single_slice)
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
        let torrent: Torrent = serde_bencode::from_bytes(&dot_torrent).unwrap();
        let info_hash = torrent.info_hash();

        assert_eq!(hex::encode(info_hash), "872ab5abd72ea223d2a2e36688cc96f83bb71d42");
    }
}
